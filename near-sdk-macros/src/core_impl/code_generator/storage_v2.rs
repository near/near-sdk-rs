//! Code generation for `#[near(storage)]` records and `#[near(contract_state(layout = v2))]`
//! roots (storage v2 prototype, see PROTOTYPE_NOTES.md at the repo root).
//!
//! For a record `S` with `#[key(n)]` on every named field this emits:
//! - the struct itself with the `#[key]` attributes stripped;
//! - `impl Value for S`: tagged-record encode (merging unknown tags from the stored bytes),
//!   decode (missing tag -> `FieldDefault`, or an error for `#[key(n, required)]`),
//!   bind / detach, and `MAX_SUBPATH` summed over fields;
//! - `SView<'a>` plus `impl Record for S` and `impl ViewField for S`: one field per struct
//!   field, typed `<T as ViewField<'a, S>>::Out` (an `At` handle for collections, the nested
//!   view for records, `UseGetForScalars` for scalars);
//! - `impl Removable for S where for<'x> T: Removable, ...` (HRTB so a non-removable field makes
//!   the impl inapplicable instead of a compile error at the struct);
//! - `impl Default for S` (unbound) when no field is `required` and the user did not derive one.
//!
//! For a root: `impl Root` (encode, decode, bind), `impl Default` (decode of an empty record,
//! unless derived), and the `ContractState` / `StateRead` / `StateWrite` impls from
//! [`contract_state_v2`].

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::{Fields, ItemStruct, LitInt, spanned::Spanned};

pub struct V2Options {
    /// `Some(root byte expr)` for a contract root.
    pub root: Option<TokenStream>,
    pub reserved: Vec<u32>,
}

struct KeyedField {
    ident: syn::Ident,
    ty: syn::Type,
    vis: syn::Visibility,
    tag: u32,
    required: bool,
}

/// Collection names that exist only in v1 (`near_sdk::store` / `near_sdk::collections`). A
/// record (not the root) holding one is rejected: v1 collections belong at the root.
const V1_ONLY_NAMES: &[&str] = &[
    "IterableMap",
    "IterableSet",
    "UnorderedMap",
    "UnorderedSet",
    "TreeMap",
    "LegacyTreeMap",
    "LazyOption",
];

/// Names shared by v1 and v2 (`LookupMap`, `LookupSet`, `Vector`, `Lazy`): rejected only when
/// the path names the v1 module (`store::LookupMap`, `collections::Vector`). Best effort: an
/// unqualified import of a v1 `LookupMap`, a renamed import or a type alias slips through.
const SHARED_NAMES: &[&str] = &["LookupMap", "LookupSet", "Vector", "Lazy"];

fn v1_name(ty: &syn::Type) -> Option<String> {
    let syn::Type::Path(tp) = ty else { return None };
    let last = tp.path.segments.last()?.ident.to_string();
    if V1_ONLY_NAMES.contains(&last.as_str()) {
        return Some(last);
    }
    let v1_module =
        tp.path.segments.iter().any(|s| s.ident == "store" || s.ident == "collections");
    (v1_module && SHARED_NAMES.contains(&last.as_str())).then_some(last)
}

/// v2 collection names, for the "collection as a map value" check.
const V2_COLLECTIONS: &[&str] =
    &["Map", "LookupMap", "Set", "LookupSet", "Vector", "Lazy", "GroupedMap"];

/// `Map<_, X>`, `LookupMap<_, X>`, `GroupedMap<_, _, X>`, `Vector<X>`, `Lazy<X>` where `X` is
/// a collection: returns `X`'s span. Syntactic, so an alias slips through to the
/// post-monomorphization check, which stays as the backstop.
fn collection_valued(ty: &syn::Type) -> Option<Span> {
    let syn::Type::Path(tp) = ty else { return None };
    let seg = tp.path.segments.last()?;
    let syn::PathArguments::AngleBracketed(args) = &seg.arguments else { return None };
    let tys: Vec<&syn::Type> = args
        .args
        .iter()
        .filter_map(|a| if let syn::GenericArgument::Type(t) = a { Some(t) } else { None })
        .collect();
    let value = match seg.ident.to_string().as_str() {
        "Map" | "LookupMap" => tys.get(1),
        "GroupedMap" => tys.get(2),
        "Vector" | "Lazy" => tys.first(),
        _ => None,
    }?;
    let syn::Type::Path(vp) = value else { return None };
    let name = vp.path.segments.last()?.ident.to_string();
    V2_COLLECTIONS.contains(&name.as_str()).then(|| value.span())
}

fn derives(item: &ItemStruct, names: &[&str]) -> bool {
    item.attrs.iter().any(|a| {
        if !a.path().is_ident("derive") {
            return false;
        }
        let mut hit = false;
        let _ = a.parse_nested_meta(|meta| {
            if let Some(last) = meta.path.segments.last()
                && names.iter().any(|n| last.ident == n)
            {
                hit = true;
            }
            Ok(())
        });
        hit
    })
}

fn parse_key_attr(attr: &syn::Attribute) -> syn::Result<(u32, bool)> {
    // `#[key(3)]` / `#[key(3, required)]`
    let args = attr.parse_args_with(
        syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated,
    )?;
    let mut it = args.iter();
    let tag = match it.next() {
        Some(syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Int(i), .. })) => i.base10_parse::<u32>()?,
        other => {
            return Err(syn::Error::new(
                other.map_or(attr.span(), |e| e.span()),
                "expected `#[key(n)]` with an integer tag",
            ));
        }
    };
    let mut required = false;
    for e in it {
        match e {
            syn::Expr::Path(p) if p.path.is_ident("required") => required = true,
            _ => return Err(syn::Error::new(e.span(), "expected `required`")),
        }
    }
    if tag == 0 {
        return Err(syn::Error::new(attr.span(), "tag 0 is reserved; tags start at 1"));
    }
    Ok((tag, required))
}

pub fn expand(
    mut item: ItemStruct,
    opts: V2Options,
    struct_attrs: TokenStream,
) -> syn::Result<TokenStream> {
    if !item.generics.params.is_empty() {
        return Err(syn::Error::new(
            item.generics.span(),
            "storage v2 prototype: generic records are not supported",
        ));
    }
    let is_root = opts.root.is_some();
    let Fields::Named(named) = &mut item.fields else {
        return Err(syn::Error::new(item.span(), "storage records need named fields"));
    };

    let mut fields = Vec::new();
    for f in named.named.iter_mut() {
        let mut found = None;
        let mut err = None;
        f.attrs.retain(|a| {
            if a.path().is_ident("key") {
                match parse_key_attr(a) {
                    Ok(k) => found = Some(k),
                    Err(e) => err = Some(e),
                }
                false
            } else {
                true
            }
        });
        if let Some(e) = err {
            return Err(e);
        }
        let ident = f.ident.clone().expect("named");
        let Some((tag, required)) = found else {
            return Err(syn::Error::new(
                ident.span(),
                format!("field `{ident}` needs a `#[key(n)]` tag: the tags are the storage layout"),
            ));
        };
        if opts.reserved.contains(&tag) {
            return Err(syn::Error::new(ident.span(), format!("tag {tag} is reserved")));
        }
        if let Some(prev) = fields.iter().find(|k: &&KeyedField| k.tag == tag) {
            return Err(syn::Error::new(
                ident.span(),
                format!("tag {tag} is already used by `{}`", prev.ident),
            ));
        }
        if !is_root && let Some(n) = v1_name(&f.ty) {
            return Err(syn::Error::new(
                f.ty.span(),
                format!(
                    "v1 collection `{n}` inside a storage record: v1 collections belong at the \
                     contract root (their write-on-drop and removal do not compose with records)"
                ),
            ));
        }
        if v1_name(&f.ty).is_none()
            && let Some(span) = collection_valued(&f.ty)
        {
            return Err(syn::Error::new(
                span,
                "a map value cannot be a collection; put it in a `#[near(storage)]` record, or \
                 use `GroupedMap`",
            ));
        }
        fields.push(KeyedField { ident, ty: f.ty.clone(), vis: f.vis.clone(), tag, required });
    }

    let name = &item.ident;
    let vis = &item.vis;
    let view = format_ident!("{}At", name);
    let view_mut = format_ident!("{}AtMut", name);
    let s = quote!(::near_sdk::state);
    let p = quote!(::near_sdk::state::__private);
    let tag_lit = |t: u32| LitInt::new(&format!("{t}u32"), Span::call_site());

    let mut max_terms = Vec::new();
    let mut enc = Vec::new();
    let mut known_arms = Vec::new();
    let mut dec = Vec::new();
    let mut bind = Vec::new();
    let mut detach = Vec::new();
    let mut view_fields = Vec::new();
    let mut view_inits = Vec::new();
    let mut view_mut_fields = Vec::new();
    let mut view_mut_inits = Vec::new();
    let mut removable_bounds = Vec::new();
    let mut clear = Vec::new();

    for KeyedField { ident, ty, vis: fvis, tag, required } in &fields {
        let t = tag_lit(*tag);
        max_terms.push(quote! { m = #p::max(m, #p::field_subpath::<#ty>(#t)); });
        enc.push(quote! { #p::encode_field(&mut w, of.as_ref(), #t, &self.#ident); });
        known_arms.push(quote! { #t => !<#ty as #s::Value>::INLINE, });
        // Spanned at the field type, so a missing `Default` points at the field.
        if *required {
            dec.push(quote_spanned! {ty.span()=> #ident: #p::field_required::<#ty>(f, #t, path)?, });
        } else {
            dec.push(quote_spanned! {ty.span()=> #ident: #p::field::<#ty>(f, #t, path)?, });
        }
        bind.push(quote! { #s::Value::bind(&mut self.#ident, &path.field(#t)); });
        detach.push(quote! { #s::Value::detach(&mut self.#ident); });
        view_fields.push(quote! { #fvis #ident: <#ty as #s::ViewField<'a, #name>>::Out, });
        view_inits.push(quote! {
            #ident: <#ty as #s::ViewField<'a, #name>>::__view_field(path.field(#t)),
        });
        view_mut_fields
            .push(quote! { #fvis #ident: <#ty as #s::ViewFieldMut<'a, #name>>::Out, });
        view_mut_inits.push(quote! {
            #ident: <#ty as #s::ViewFieldMut<'a, #name>>::__view_field_mut(path.field(#t)),
        });
        removable_bounds.push(quote! { for<'__x> #ty: #s::Removable, });
        clear.push(quote! { <#ty as #s::Removable>::clear_children(&path.field(#t)); });
    }
    let reserved_arms = opts.reserved.iter().map(|r| {
        let t = tag_lit(*r);
        quote! { #t => false, }
    });

    let max_subpath = quote! {{
        let mut m = 0usize;
        #(#max_terms)*
        m
    }};
    let encode_body = quote! {
        let of = #p::parse_orig(orig);
        let mut w = #p::RecordWriter::new();
        #(#enc)*
        // keep a stored tag iff the code does not know it inline and it is not reserved
        w.finish(orig, |t| match t {
            #(#reserved_arms)*
            #(#known_arms)*
            _ => true,
        })
    };
    // The one place each field's decode (and its `FieldDefault` bound) is spelled, shared by
    // `decode_value` and the generated `Default`, so a missing `Default` is reported once.
    let decode_fields = quote! {
        impl #name {
            #[doc(hidden)]
            #[allow(dead_code)]
            fn __decode_fields(
                f: &#p::Fields<'_>,
                path: ::core::option::Option<&#s::Path>,
            ) -> ::core::result::Result<Self, #s::RecordError> {
                ::core::result::Result::Ok(Self { #(#dec)* })
            }
        }
    };
    let decode_body = quote! {
        let f = #p::parse(bytes)?;
        Self::__decode_fields(&f, ::core::option::Option::Some(path))
    };
    let user_default = derives(&item, &["Default", "PanicOnDefault", "SmartDefault"]);

    if let Some(root) = &opts.root {
        // A root never returns `None` from `state_read`, but the wrapper's `unwrap_or_default()`
        // still needs `Default`: decode an empty record at the root path (bound collections;
        // panics on a `required` field).
        let default_impl = (!user_default).then(|| {
            quote! {
                impl ::core::default::Default for #name {
                    fn default() -> Self {
                        let path = <Self as #s::Root>::root_path();
                        <Self as #s::Root>::__decode_root(&[], &path).unwrap_or_else(|e| {
                            ::near_sdk::env::panic_str(&::std::format!(
                                "state: `Default` for a v2 root decodes an empty record: {e}"
                            ))
                        })
                    }
                }
            }
        });
        return Ok(quote! {
            #struct_attrs
            #item

            const _: () = {
                impl #s::Root for #name {
                    const ROOT_BYTE: u8 = #root;
                    const MAX_SUBPATH: usize = #max_subpath;
                    fn __encode_root(&self, orig: ::core::option::Option<&[u8]>) -> ::std::vec::Vec<u8> {
                        #encode_body
                    }
                    fn __decode_root(bytes: &[u8], path: &#s::Path)
                        -> ::core::result::Result<Self, #s::RecordError>
                    {
                        #decode_body
                    }
                    fn __bind_root(&mut self, path: &#s::Path) {
                        #(#bind)*
                    }
                }
                #decode_fields
                #default_impl
            };

            impl #name {
                /// Binds the root's collections to their paths, for an `#[init]` body that
                /// seeds them: `let mut this = Self { .. }.bind();`. Idempotent; the wrapper
                /// binds the returned root anyway.
                #[track_caller]
                pub fn bind(self) -> Self {
                    <Self as #s::Root>::bind(self)
                }
            }
        });
    }

    let has_required = fields.iter().any(|f| f.required);
    let default_impl = (!user_default && !has_required).then(|| {
        quote! {
            impl ::core::default::Default for #name {
                fn default() -> Self {
                    // no `required` field, so decoding an empty record cannot fail
                    match Self::__decode_fields(&::std::vec::Vec::new(), ::core::option::Option::None) {
                        ::core::result::Result::Ok(v) => v,
                        ::core::result::Result::Err(e) => ::core::panic!("state: {e}"),
                    }
                }
            }
        }
    });

    let view_doc = format!(
        "Read-only view of a [`{name}`] record, generated by `#[near(storage)]` and returned by \
         `map.at(&k)`. Its collection and `Lazy` fields are read-only handles bound to the \
         record's path; nothing is read until you call a method on one. Scalar fields are not \
         readable here (they are typed `UseGetForScalars`): use `map.get(&k)` for them."
    );
    let view_mut_doc = format!(
        "Mutable view of a [`{name}`] record, generated by `#[near(storage)]` and returned by \
         `map.at_mut(&k)` (1 `storage_has_key`, panics if the record is absent). Its collection \
         and `Lazy` fields are `&mut` handles bound to the record's path; the record itself is \
         not read. Scalar fields are not available (typed `UseGetMutForScalars`): use \
         `map.get_mut(&k)` to change them."
    );

    Ok(quote! {
        #struct_attrs
        #item

        #[doc = #view_doc]
        #[allow(dead_code)]
        #vis struct #view<'a> {
            #(#view_fields)*
            #[doc(hidden)]
            pub __m: ::core::marker::PhantomData<&'a ()>,
        }

        #[doc = #view_mut_doc]
        #[allow(dead_code)]
        #vis struct #view_mut<'a> {
            #(#view_mut_fields)*
            #[doc(hidden)]
            pub __m: ::core::marker::PhantomData<&'a mut ()>,
        }

        const _: () = {
            impl #s::Value for #name {
                const MAX_SUBPATH: usize = #max_subpath;
                fn encode_value(&self, orig: ::core::option::Option<&[u8]>) -> ::std::vec::Vec<u8> {
                    #encode_body
                }
                fn decode_value(bytes: &[u8], path: &#s::Path)
                    -> ::core::result::Result<Self, #s::RecordError>
                {
                    #decode_body
                }
                fn bind(&mut self, path: &#s::Path) {
                    #(#bind)*
                }
                fn detach(&mut self) {
                    #(#detach)*
                }
            }

            impl #s::Record for #name {
                type View<'a> = #view<'a>;
                type ViewMut<'a> = #view_mut<'a>;
                fn __view<'a>(path: #s::Path) -> #view<'a> {
                    #view { #(#view_inits)* __m: ::core::marker::PhantomData }
                }
                fn __view_mut<'a>(path: #s::Path) -> #view_mut<'a> {
                    #view_mut { #(#view_mut_inits)* __m: ::core::marker::PhantomData }
                }
            }

            impl<'a, __R: #s::Value> #s::ViewField<'a, __R> for #name {
                type Out = #view<'a>;
                fn __view_field(field_path: #s::Path) -> #view<'a> {
                    <#name as #s::Record>::__view(field_path)
                }
            }

            impl<'a, __R: #s::Value> #s::ViewFieldMut<'a, __R> for #name {
                type Out = #view_mut<'a>;
                fn __view_field_mut(field_path: #s::Path) -> #view_mut<'a> {
                    <#name as #s::Record>::__view_mut(field_path)
                }
            }

            #[allow(unused_lifetimes)]
            impl #s::Removable for #name where #(#removable_bounds)* {
                fn clear_children(path: &#s::Path) {
                    #(#clear)*
                }
            }

            #decode_fields
            #default_impl
        };
    })
}

/// `ContractState` and the wrapper hooks for a v2 root. No Borsh impls: the wrapper's
/// `state_read()` / `state_write()` are bounded on `StateRead` / `StateWrite`, which v1 roots
/// get from a Borsh blanket and v2 roots get here.
pub fn contract_state_v2(ident: &syn::Ident, root: &TokenStream) -> TokenStream {
    quote! {
        const _: () = {
            impl ::near_sdk::state::ContractState for #ident {
                #[inline]
                fn state_key() -> &'static [u8] {
                    const KEY: [u8; 1] = [#root];
                    &KEY
                }
                fn state_exists() -> bool {
                    ::near_sdk::state::root_exists::<Self>()
                }
            }
            impl ::near_sdk::state::StateRead for #ident {
                fn read_state(_key: &[u8]) -> ::core::option::Option<Self> {
                    ::core::option::Option::Some(::near_sdk::state::load_root::<Self>())
                }
            }
            impl ::near_sdk::state::StateWrite for #ident {
                fn write_state(&self, _key: &[u8]) -> bool {
                    ::near_sdk::state::flush_root(self);
                    true
                }
            }
        };
    }
}
