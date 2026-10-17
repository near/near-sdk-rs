//! Code generation for `#[near(storage)]` records and `#[near(contract_state(layout = v2))]`
//! roots (storage v2 prototype, see PROTOTYPE_NOTES.md at the repo root).
//!
//! For a struct `S` with `#[key(n)]` on every named field this emits:
//! - the struct itself with the `#[key]` attributes stripped;
//! - `impl Value for S`: tagged-record encode (merging unknown tags from the stored bytes),
//!   decode (missing tag -> `Default`, or an error for `#[key(n, required)]`), bind / unbind,
//!   and `MAX_SUBPATH` summed over fields;
//! - `SView<'a>` plus `impl Record for S`: one public field per struct field, typed
//!   `<T as ViewField<'a, S>>::Out` (a bound handle for collections, a lazy `Scalar` otherwise);
//! - `impl Removable for S where for<'x> T: Removable, ...` (HRTB so a non-removable field makes
//!   the impl inapplicable instead of a compile error at the struct);
//! - for a root: `impl Root`, and the `ContractState` overrides (emitted by `state.rs`).

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
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

pub fn expand(mut item: ItemStruct, opts: V2Options, struct_attrs: TokenStream) -> syn::Result<TokenStream> {
    if !item.generics.params.is_empty() {
        return Err(syn::Error::new(
            item.generics.span(),
            "storage v2 prototype: generic records are not supported",
        ));
    }
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
        fields.push(KeyedField { ident, ty: f.ty.clone(), vis: f.vis.clone(), tag, required });
    }

    let name = &item.ident;
    let vis = &item.vis;
    let view = format_ident!("{}View", name);
    let s = quote!(::near_sdk::state);
    let p = quote!(::near_sdk::state::__private);
    let tag_lit = |t: u32| LitInt::new(&format!("{t}u32"), Span::call_site());

    let mut max_terms = Vec::new();
    let mut enc = Vec::new();
    let mut known_arms = Vec::new();
    let mut dec = Vec::new();
    let mut bind = Vec::new();
    let mut unbind = Vec::new();
    let mut view_fields = Vec::new();
    let mut view_inits = Vec::new();
    let mut removable_bounds = Vec::new();
    let mut clear = Vec::new();

    for KeyedField { ident, ty, vis: fvis, tag, required } in &fields {
        let t = tag_lit(*tag);
        max_terms.push(quote! { m = #p::max(m, #p::field_subpath::<#ty>(#t)); });
        enc.push(quote! { #p::encode_field(&mut w, of.as_ref(), #t, &self.#ident); });
        known_arms.push(quote! { #t => !<#ty as #s::Value>::INLINE, });
        if *required {
            dec.push(quote! { #ident: #p::field_required::<#ty>(&f, #t, path)?, });
        } else {
            dec.push(quote! { #ident: #p::field::<#ty>(&f, #t, path)?, });
        }
        bind.push(quote! { #s::Value::bind(&mut self.#ident, &path.field(#t)); });
        unbind.push(quote! { #s::Value::unbind(&mut self.#ident); });
        view_fields.push(quote! { #fvis #ident: <#ty as #s::ViewField<'a, #name>>::Out, });
        view_inits.push(quote! {
            #ident: <#ty as #s::ViewField<'a, #name>>::__view_field(
                &path, path.field(#t), |r: &#name| &r.#ident),
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
    let decode_body = quote! {
        let f = #p::parse(bytes)?;
        ::core::result::Result::Ok(Self { #(#dec)* })
    };

    if let Some(root) = &opts.root {
        // A root is not a `Value` (see `state::Root` docs): it only needs encode/decode.
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
                }
            };
        });
    }

    let view_doc = format!("Path-bound view of [`{name}`] returned by `at()`; reads nothing by itself.");

    Ok(quote! {
        #struct_attrs
        #item

        #[doc = #view_doc]
        #vis struct #view<'a> {
            #(#view_fields)*
            #[doc(hidden)]
            pub __m: ::core::marker::PhantomData<&'a ()>,
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
                fn unbind(&mut self) {
                    #(#unbind)*
                }
            }

            impl #s::Record for #name {
                type View<'a> = #view<'a>;
                fn __view<'a>(path: #s::Path) -> #view<'a> {
                    #view { #(#view_inits)* __m: ::core::marker::PhantomData }
                }
            }

            #[allow(unused_lifetimes)]
            impl #s::Removable for #name where #(#removable_bounds)* {
                fn clear_children(path: &#s::Path) {
                    #(#clear)*
                }
            }
        };
    })
}

/// `ContractState` overrides and placeholder Borsh impls for a v2 root.
///
/// The `#[near]` method wrapper calls `ContractState::state_read()` / `state_write()`, whose
/// default bodies carry `where Self: BorshDeserialize` / `BorshSerialize`. An override cannot
/// drop those bounds, so the root must implement Borsh even though it is never Borsh-encoded.
pub fn contract_state_v2(ident: &syn::Ident, root: &TokenStream) -> TokenStream {
    quote! {
        const _: () = {
            impl ::near_sdk::state::ContractState for #ident {
                #[inline]
                fn state_key() -> &'static [u8] {
                    const KEY: [u8; 1] = [#root];
                    &KEY
                }
                fn state_read() -> ::core::option::Option<Self>
                where
                    Self: ::near_sdk::borsh::BorshDeserialize,
                {
                    ::core::option::Option::Some(::near_sdk::state::load_root::<Self>())
                }
                fn state_write(&self) -> bool
                where
                    Self: ::near_sdk::borsh::BorshSerialize,
                {
                    ::near_sdk::state::flush_root(self);
                    true
                }
            }
            impl ::near_sdk::borsh::BorshSerialize for #ident {
                fn serialize<W: ::near_sdk::borsh::io::Write>(&self, _: &mut W)
                    -> ::near_sdk::borsh::io::Result<()>
                {
                    ::core::unreachable!("a storage v2 root is a tagged record, not Borsh")
                }
            }
            impl ::near_sdk::borsh::BorshDeserialize for #ident {
                fn deserialize_reader<R: ::near_sdk::borsh::io::Read>(_: &mut R)
                    -> ::near_sdk::borsh::io::Result<Self>
                {
                    ::core::unreachable!("a storage v2 root is a tagged record, not Borsh")
                }
            }
        };
    }
}
