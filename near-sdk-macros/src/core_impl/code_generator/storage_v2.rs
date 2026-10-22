//! Code generation for `#[near(storage)]` records and `#[near(contract_state(layout = v2))]`
//! roots (storage v2 prototype, see PROTOTYPE_NOTES.md at the repo root).
//!
//! For a record `S` with `#[key(n)]` on every named field this emits:
//! - the struct itself with the `#[key]` attributes stripped;
//! - `impl Value for S`: tagged-record encode (merging unknown tags from the stored bytes),
//!   decode (missing tag -> `FieldDefault`, the const expression of
//!   `#[key(n, default = expr)]` (emitted as a `const` item, T6), a call of the `fn() -> T` of
//!   `#[key(n, default_fn = path)]` (API round 4), or an error for `#[key(n, required)]`),
//!   bind / detach, and `MAX_SUBPATH` summed over fields; since API round 5 each collection
//!   field leaves its kind marker (a wire-1 field, `tag << 2 | 1, 01, kind`) in the record (a
//!   root's writer leaves none), a nested record field is wire 2, `__RECORD = true`, and
//!   `__unknown_collection` (the removal check: an undeclared, unreserved tag of wire 1, or a
//!   wire-2 nested record holding one at any depth; a wire-0 scalar never matches);
//! - `SAt<'a>` (from `at`) and `SAtMut<'a>` (from `at_mut`), plus `impl Record for S` and
//!   `impl ViewField / ViewFieldMut for S`: one field per struct field, typed
//!   `<T as ViewField<'a, S>>::Out` (an `At` handle for collections, the nested view for
//!   records, `UseGetForScalars` for scalars) or `<T as ViewFieldMut<'a, S>>::Out` (`&'a mut`
//!   handles from the cache arena, `UseGetMutForScalars` for scalars);
//! - `__write_pending` walks, so a root struct literal's `Lazy::new` values are written by
//!   the flush without `.bind()`;
//! - `#[key(n, legacy)]` root fields (T7): encoded and decoded as plain Borsh outside `Value`
//!   (a v1 collection's bytes are its prefix), required, no binding; rejected outside a root;
//! - every other field's type must implement `Value` (T7): the decode and encode helpers bound
//!   it, so a v1 collection or an un-opted Borsh struct is E0277 with `Value`'s message;
//! - rejections: a missing `#[key]`, a reused or reserved tag, a v1 collection in a record
//!   (v1-only names, or `store::`/`collections::`-qualified shared names), and a collection
//!   as a map value (`Map<_, Set<_>>` and friends) at the field; and (round 7) a headerless
//!   collection (`LookupMap`, `LookupSet`, `GroupedMap`) as a field of a record, by a const
//!   assertion on `Value::__ROOT_ONLY` spanned at the field type (E0080): root-only, since they
//!   give the A3 first-write rule no signal;
//! - `SAtMut` handles remember the stored record they sit in (`__view_mut_in(path, anchor)`,
//!   the anchor passed down through nested record fields), for the A3 first-write rule;
//! - `impl Removable for S where for<'x> T: Removable, ...` (HRTB so a non-removable field makes
//!   the impl inapplicable instead of a compile error at the struct);
//! - `impl Leaf for S where for<'x> T: Leaf, ...` (same trick): plain `insert` only for records
//!   that own no collections;
//! - `impl Default for S` (unbound) when no field is `required` and the user did not derive one.
//!
//! For a root: `impl Root` (encode, decode, bind), `impl Default` (decode of an empty record,
//! unless derived), and the `ContractState` / `StateRead` / `StateWrite` impls from
//! [`contract_state_v2`].

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::{Fields, ItemStruct, spanned::Spanned};

pub struct V2Options {
    /// `Some(root byte expr)` for a contract root.
    pub root: Option<TokenStream>,
    pub reserved: Vec<u32>,
}

struct KeyedField {
    ident: syn::Ident,
    ty: syn::Type,
    vis: syn::Visibility,
    key: KeyAttr,
}

struct KeyAttr {
    tag: u32,
    required: bool,
    /// `#[key(n, default = expr)]`: evaluated when the tag is missing from a stored record.
    default: Option<syn::Expr>,
    /// `#[key(n, default_fn = path)]`: a `fn() -> T` called when the tag is missing.
    default_fn: Option<syn::Path>,
    /// `#[key(n, legacy)]`: a v1 collection (or any opaque Borsh value) kept at the root,
    /// stored inline as its Borsh bytes without the `Value` trait. Root only; implies required.
    legacy: bool,
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

fn parse_key_attr(attr: &syn::Attribute) -> syn::Result<KeyAttr> {
    // `#[key(3)]` / `#[key(3, required)]` / `#[key(3, default = expr)]` /
    // `#[key(3, default_fn = path)]` / `#[key(3, legacy)]`
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
    let mut default = None;
    let mut default_fn = None;
    let mut legacy = false;
    for e in it {
        match e {
            syn::Expr::Path(p) if p.path.is_ident("required") => required = true,
            syn::Expr::Path(p) if p.path.is_ident("legacy") => legacy = true,
            syn::Expr::Assign(a)
                if matches!(&*a.left, syn::Expr::Path(p) if p.path.is_ident("default")) =>
            {
                default = Some((*a.right).clone());
            }
            syn::Expr::Assign(a)
                if matches!(&*a.left, syn::Expr::Path(p) if p.path.is_ident("default_fn")) =>
            {
                match &*a.right {
                    syn::Expr::Path(p) if p.qself.is_none() => default_fn = Some(p.path.clone()),
                    other => {
                        return Err(syn::Error::new(
                            other.span(),
                            "`default_fn` takes a path to a `fn() -> T`, not an expression: \
                             write `default_fn = my_default` (for a const value use \
                             `default = expr`)",
                        ));
                    }
                }
            }
            _ => {
                return Err(syn::Error::new(
                    e.span(),
                    "expected `required`, `legacy`, `default = expr` or `default_fn = path`",
                ));
            }
        }
    }
    if default.is_some() && default_fn.is_some() {
        return Err(syn::Error::new(
            attr.span(),
            "`default = ..` and `default_fn = ..` are exclusive: `default` is a const value, \
             `default_fn` a function called on every decode where the tag is absent",
        ));
    }
    let has_default = default.is_some() || default_fn.is_some();
    if required && has_default {
        return Err(syn::Error::new(
            attr.span(),
            "`required` and `default = ..` are exclusive: `required` fails on a missing tag, \
             `default` fills it",
        ));
    }
    if legacy && has_default {
        return Err(syn::Error::new(
            attr.span(),
            "`legacy` and `default = ..` are exclusive: a legacy (v1) field is required",
        ));
    }
    if tag == 0 {
        return Err(syn::Error::new(attr.span(), "tag 0 is reserved; tags start at 1"));
    }
    Ok(KeyAttr { tag, required, default, default_fn, legacy })
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
        let Some(key) = found else {
            return Err(syn::Error::new(
                ident.span(),
                format!("field `{ident}` needs a `#[key(n)]` tag: the tags are the storage layout"),
            ));
        };
        let tag = key.tag;
        if opts.reserved.contains(&tag) {
            return Err(syn::Error::new(ident.span(), format!("tag {tag} is reserved")));
        }
        if let Some(prev) = fields.iter().find(|k: &&KeyedField| k.key.tag == tag) {
            return Err(syn::Error::new(
                ident.span(),
                format!("tag {tag} is already used by `{}`", prev.ident),
            ));
        }
        if key.legacy && !is_root {
            return Err(syn::Error::new(
                ident.span(),
                "`#[key(n, legacy)]` is only for fields of a `contract_state(layout = v2)` root: \
                 v1 collections cannot live inside a storage record",
            ));
        }
        if let Some(n) = v1_name(&f.ty) {
            if !is_root {
                return Err(syn::Error::new(
                    f.ty.span(),
                    format!(
                        "v1 collection `{n}` inside a storage record: v1 collections are allowed \
                         only as root fields marked `#[key(n, legacy)]` (their write-on-drop and \
                         removal do not compose with records); use the `near_sdk::state` type here"
                    ),
                ));
            }
            if !key.legacy {
                return Err(syn::Error::new(
                    f.ty.span(),
                    format!(
                        "v1 collection `{n}` at the root needs `#[key({tag}, legacy)]`: v1 \
                         collections are not storage v2 values and are kept as their Borsh bytes \
                         (their prefix)"
                    ),
                ));
            }
        } else if let Some(span) = collection_valued(&f.ty) {
            return Err(syn::Error::new(
                span,
                "a map value cannot be a collection; put it in a `#[near(storage)]` record \
                 (`Map<MarketId, Market>` with `Market { orders: Map<OrderId, Order> }`)",
            ));
        }
        fields.push(KeyedField { ident, ty: f.ty.clone(), vis: f.vis.clone(), key });
    }

    let name = &item.ident;
    let vis = &item.vis;
    let view = format_ident!("{}At", name);
    let view_mut = format_ident!("{}AtMut", name);
    let s = quote!(::near_sdk::state);
    let p = quote!(::near_sdk::state::__private);

    // `reserved(n)` tags, like legacy fields, are known-and-empty: not kept from the stored bytes
    // and never a removal blocker (the version that owned a reserved collection tag cleared it).
    let mut known_arms: Vec<_> = opts.reserved.iter().map(|t| quote! { #t => false, }).collect();
    let mut unknown_arms: Vec<_> = opts
        .reserved
        .iter()
        .map(|t| quote! { #t => ::core::option::Option::None, })
        .collect();
    let mut enc = Vec::new();
    let mut dec = Vec::new();
    let mut root_only = Vec::new();

    for KeyedField { ident, ty, key, .. } in &fields {
        let KeyAttr { tag: t, required, default, default_fn, legacy } = key;
        // Spanned at the field type, so a missing `Default` points at the field.
        let span = ty.span();
        let decode = if *legacy {
            quote_spanned! {span=> #p::legacy_field::<#ty>(f, #t) }
        } else if *required {
            quote_spanned! {span=> #p::field_required::<#ty>(f, #t, path) }
        } else if let Some(d) = default {
            // T6: `expr` is a `const` item, so it is evaluated at compile time and a non-const
            // expression (`env::predecessor_account_id()`, `vec![..]`, `.parse()`) is a compile
            // error at the expression (E0015). Re-evaluating it on every decode of an old
            // record is then harmless: it is the same value every time.
            let c = quote_spanned! {d.span()=> const __KEY_DEFAULT: #ty = #d; };
            quote_spanned! {span=> #p::field_or::<#ty>(f, #t, path, || { #c __KEY_DEFAULT }) }
        } else if let Some(fp) = default_fn {
            // `default_fn = path` (API round 4): the escape hatch from T6's const rule, for a
            // type with no const constructor (`AccountId`, `Vec`). Coerced to `fn() -> T`, so
            // a closure, a method needing arguments or a wrong return type is an error at the
            // path. Called on EVERY decode where the tag is absent, until some other change
            // rewrites the record (the flush writes the tag then); it must be pure.
            let c = quote_spanned! {fp.span()=> let __key_default_fn: fn() -> #ty = #fp; };
            quote_spanned! {span=> #p::field_or::<#ty>(f, #t, path, || { #c __key_default_fn() }) }
        } else {
            quote_spanned! {span=> #p::field::<#ty>(f, #t, path) }
        };
        dec.push(quote_spanned! {span=> #ident: #decode?, });
        if *legacy {
            // T7: a root-only v1 field, stored inline as its Borsh bytes (its prefix), outside
            // the `Value` trait. Owns nothing the v2 cache knows about: no subpath, no binding.
            enc.push(quote! { #p::encode_legacy_field(&mut w, #t, &self.#ident); });
            known_arms.push(quote! { #t => false, });
            unknown_arms.push(quote! { #t => ::core::option::Option::None, });
            continue;
        }
        enc.push(quote! { #p::encode_field(&mut w, of.as_ref(), #t, &self.#ident); });
        known_arms.push(quote! { #t => !<#ty as #s::Value>::INLINE, });
        // a declared field: a nested record checks its own bytes, anything else is known
        unknown_arms.push(quote! { #t => #p::declared_field_collection::<#ty>(w, b), });
        if !is_root {
            // Round 7 (A3 first-write rule): a headerless collection has no first-write signal,
            // so it is root-only. A const check, not a syntactic one: it sees through aliases,
            // and a shadowing `use near_sdk::store::LookupMap` still fails first with
            // `Value`'s E0277 (a v1 collection is no value at all).
            let msg = format!(
                "field `{ident}` of `#[near(storage)]` record `{name}`: `LookupMap`, `LookupSet` \
                 and `GroupedMap` can only be fields of the contract root \
                 (`contract_state(layout = v2)`). They have no header, so the first write below \
                 a record cannot tell whether the record declares them, and another version of \
                 the contract could then remove the record and leave their keys behind (hazard \
                 A3). Use `Map` (for `LookupMap`) or `Set` (for `LookupSet`) inside a record. \
                 (`Migrating` is root-only too: it holds a v1 map.)"
            );
            root_only.push(quote_spanned! {ty.span()=>
                const _: () = ::core::assert!(!<#ty as #s::Value>::__ROOT_ONLY, #msg);
            });
        }
    }
    // The rest is per `Value` field only: a legacy field owns no storage the v2 cache knows.
    let v2: Vec<_> = fields.iter().filter(|f| !f.key.legacy).collect();
    let ident: Vec<_> = v2.iter().map(|f| &f.ident).collect();
    let ty: Vec<_> = v2.iter().map(|f| &f.ty).collect();
    let tag: Vec<_> = v2.iter().map(|f| f.key.tag).collect();
    let fvis: Vec<_> = v2.iter().map(|f| &f.vis).collect();
    let bind = quote! { #(#s::Value::bind(&mut self.#ident, &path.field(#tag));)* };
    let pending = quote! { #(#s::Value::__write_pending(&self.#ident, &path.field(#tag));)* };
    let unknown_collection_body = quote! {
        let f = #p::parse(bytes).ok()?;
        for &(t, w, b) in f.iter() {
            let hit = match t {
                #(#unknown_arms)*
                _ => #p::unknown_field_collection(w, b),
            };
            if let ::core::option::Option::Some((mut trail, k)) = hit {
                trail.insert(0, t);
                return ::core::option::Option::Some((trail, k));
            }
        }
        ::core::option::Option::None
    };

    let max_subpath = quote! {{
        let mut m = 0usize;
        #(m = #p::max(m, #p::field_subpath::<#ty>(#tag));)*
        m
    }};
    // A root never leaves kind markers (it is never removed); a record does (API round 5).
    let writer = if is_root { quote!(#p::RecordWriter::root()) } else { quote!(#p::RecordWriter::new()) };
    let encode_body = quote! {
        let of = #p::parse_orig(orig);
        let mut w = #writer;
        #(#enc)*
        // keep a stored tag iff the code does not know it inline and it is not reserved
        w.finish(orig, |t| match t {
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
                        #bind
                    }
                    fn __write_pending_root(&self, path: &#s::Path) {
                        #pending
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

    let has_required = fields.iter().any(|f| f.key.required);
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
         `map.at_mut(&k)` (1 `storage_has_key`; `None` if the record is absent). Its collection \
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
            #(#fvis #ident: <#ty as #s::ViewField<'a, #name>>::Out,)*
            #[doc(hidden)]
            pub __m: ::core::marker::PhantomData<&'a ()>,
        }

        #[doc = #view_mut_doc]
        #[allow(dead_code)]
        #vis struct #view_mut<'a> {
            #(#fvis #ident: <#ty as #s::ViewFieldMut<'a, #name>>::Out,)*
            #[doc(hidden)]
            pub __m: ::core::marker::PhantomData<&'a mut ()>,
        }

        const _: () = {
            impl #s::Value for #name {
                const MAX_SUBPATH: usize = #max_subpath;
                const __RECORD: bool = true;
                fn __unknown_collection(bytes: &[u8])
                    -> ::core::option::Option<(::std::vec::Vec<u32>, u8)>
                {
                    #unknown_collection_body
                }
                fn encode_value(&self, orig: ::core::option::Option<&[u8]>) -> ::std::vec::Vec<u8> {
                    #encode_body
                }
                fn decode_value(bytes: &[u8], path: &#s::Path)
                    -> ::core::result::Result<Self, #s::RecordError>
                {
                    #decode_body
                }
                fn bind(&mut self, path: &#s::Path) {
                    #bind
                }
                fn detach(&mut self) {
                    #(#s::Value::detach(&mut self.#ident);)*
                }
                fn __write_pending(&self, path: &#s::Path) {
                    #pending
                }
            }

            impl #s::Record for #name {
                type View<'a> = #view<'a>;
                type ViewMut<'a> = #view_mut<'a>;
                fn __view<'a>(path: #s::Path) -> #view<'a> {
                    #view {
                        #(#ident: <#ty as #s::ViewField<'a, #name>>::__view_field(path.field(#tag)),)*
                        __m: ::core::marker::PhantomData
                    }
                }
                #[allow(unused_variables)]
                fn __view_mut_in<'a>(path: #s::Path, anchor: usize) -> #view_mut<'a> {
                    #view_mut {
                        #(#ident: <#ty as #s::ViewFieldMut<'a, #name>>::__view_field_mut(
                            path.field(#tag),
                            anchor,
                        ),)*
                        __m: ::core::marker::PhantomData
                    }
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
                fn __view_field_mut(field_path: #s::Path, anchor: usize) -> #view_mut<'a> {
                    // a nested record field: its collections live in the enclosing stored record
                    <#name as #s::Record>::__view_mut_in(field_path, anchor)
                }
            }

            #[allow(unused_lifetimes)]
            impl #s::Removable for #name where #(for<'__x> #ty: #s::Removable,)* {
                fn clear_children(path: &#s::Path) {
                    #(<#ty as #s::Removable>::clear_children(&path.field(#tag));)*
                }
                fn clear_children_some(path: &#s::Path, budget: &mut u32) -> bool {
                    #(if !<#ty as #s::Removable>::clear_children_some(&path.field(#tag), budget) {
                        return false;
                    })*
                    true
                }
            }

            // D6: a record owns no storage iff every field is a `Leaf`; plain `insert` (which
            // overwrites) is only available for such records.
            #[allow(unused_lifetimes)]
            impl #s::Leaf for #name where #(for<'__x> #ty: #s::Leaf,)* {}

            #(#root_only)*
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
