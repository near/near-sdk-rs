//! Code generation for `#[near(storage)]` records and `#[near(contract_state(layout = v2))]`
//! roots (storage v2 prototype, see PROTOTYPE_NOTES.md at the repo root).
//!
//! For a record `S` with `#[key(n)]` on every named field this emits, besides `S` itself with
//! the `#[key]` attributes stripped:
//! - `impl Value`: tagged-record encode (keeping unknown stored tags; each collection field
//!   leaves its kind marker, a nested record is wire 2), decode (a missing tag is
//!   `FieldDefault`, the associated `const` of `default = expr` (forced, so evaluated by
//!   `cargo check`), a call of `default_fn = path`, or an
//!   error for `required`), bind / detach / `__write_pending` (so a struct literal's
//!   `Lazy::new` values are written without `.bind()`), `MAX_SUBPATH` summed over fields, and
//!   `__unknown_collection`, the removal check: a collection marker under an undeclared,
//!   unreserved tag, at any depth of nested records;
//! - `SAt<'a>` / `SAtMut<'a>` views (from `at` / `at_mut`) with `impl Record` and
//!   `impl ViewField / ViewFieldMut`: one field per struct field, a handle for collections, the
//!   nested view for records, unavailable for scalars. `SAtMut` handles carry the anchor of the
//!   stored record they sit in, for the A3 first-write rule;
//! - `impl Removable` and `impl Leaf` with `for<'x> T: ..` bounds (HRTB), so a field without
//!   the trait makes the impl inapplicable instead of a compile error at the struct;
//! - `impl Default` (unbound) unless a field is `required` or the user derives one;
//! - per field, a const assertion on `Value::__ROOT_ONLY` (E0080 at the field type): the
//!   headerless `LookupMap`, `LookupSet` and `GroupedMap` give the A3 first-write rule no
//!   signal, so they are root-only.
//!
//! The derive rejects a missing `#[key]`, a reused or reserved tag, a v1 collection in a
//! record (by name, best effort) and a collection as a map value. Any other field type must
//! implement `Value`, so the rest (an aliased v1 collection, an un-opted Borsh struct) is E0277
//! with `Value`'s message.
//!
//! A root gets `impl Root` (encode without kind markers, decode, bind), `impl Default` (decode
//! of an empty record, unless derived), and the `ContractState` / `StateRead` / `StateWrite`
//! impls from [`contract_state_v2`]. Only a root takes `#[key(n, legacy)]`: a v1 collection
//! stored as plain Borsh outside `Value` (its bytes are its prefix), required, never bound.
//!
//! Hygiene: user tokens (field types, `default`, `default_fn`, the root byte) never resolve to
//! something the derive generates, and the generated code never resolves to a user item.
//! - Every binding (parameter, `let`, closure or loop pattern) is mixed-site, so no user token
//!   sees it, and is named `__near_*`: an identifier pattern resolves items at the call site
//!   even when mixed-site, so a user `const` or unit struct of the same name would capture it
//!   as a constant pattern. A refutable pattern with a binding is an exhaustive `match`, so
//!   even then the capture is a compile error, not a silently skipped arm.
//! - The views' phantom field and the `ViewField` impls' parent parameter are named apart from
//!   the record's fields and name; `state_key` reads `Root::ROOT_BYTE`, so the root byte is
//!   spelled once, in an impl, next to no generated name.
//! - The `__near_` prefix (any case) is reserved: the bindings above and the inherent items the
//!   derive adds to the record (`__near_decode_fields`, `__NEAR_KEY_DEFAULT_<tag>`) use it.

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
    let mut tys = args
        .args
        .iter()
        .filter_map(|a| if let syn::GenericArgument::Type(t) = a { Some(t) } else { None });
    let value = match seg.ident.to_string().as_str() {
        "Map" | "LookupMap" => tys.nth(1),
        "GroupedMap" => tys.nth(2),
        "Vector" | "Lazy" => tys.next(),
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

/// A binding of the generated code (a parameter, `let`, closure or loop pattern). Mixed-site, so
/// no user token (a default, a field type, the root byte) resolves to it; `__near_`-prefixed,
/// because hygiene does not cover the other direction: an identifier PATTERN resolves items at
/// the call site even when mixed-site, so a user `const` or unit struct of the same name would
/// turn it into a constant pattern.
fn local(name: &str) -> syn::Ident {
    syn::Ident::new(&format!("__near_{name}"), Span::mixed_site())
}

/// `base` with `_` appended until it is none of `taken`.
fn unused_ident<'a>(
    base: &str,
    taken: impl IntoIterator<Item = &'a syn::Ident> + Clone,
) -> syn::Ident {
    let mut name = base.to_owned();
    while taken.clone().into_iter().any(|t| *t == name) {
        name.push('_');
    }
    syn::Ident::new(&name, Span::call_site())
}

pub fn expand(
    mut item: ItemStruct,
    opts: V2Options,
    struct_attrs: Option<TokenStream>,
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
    let mut unknown_arms: Vec<_> =
        opts.reserved.iter().map(|t| quote! { #t => ::core::option::Option::None, }).collect();
    // every binding of the template; see `local`
    let [l_bytes, l_orig, l_of, l_w, l_f, l_path, l_t, l_b, l_hit, l_trail, l_k] =
        ["bytes", "orig", "of", "w", "f", "path", "t", "b", "hit", "trail", "k"].map(local);
    let [l_anchor, l_budget, l_v, l_e] = ["anchor", "budget", "v", "e"].map(local);
    let mut enc = Vec::new();
    let mut dec = Vec::new();
    let (mut defaults, mut defaults_forced) = (Vec::new(), Vec::new());
    let mut root_only = Vec::new();

    for KeyedField { ident, ty, key, .. } in &fields {
        let KeyAttr { tag: t, required, default, default_fn, legacy } = key;
        // Spanned at the field type, so a missing `Default` points at the field.
        let span = ty.span();
        let decode = if *legacy {
            quote_spanned! {span=> #p::legacy_field::<#ty>(#l_f, #t) }
        } else if *required {
            quote_spanned! {span=> #p::field_required::<#ty>(#l_f, #t, #l_path) }
        } else if let Some(d) = default {
            // `expr` is an associated `const` of the record, so it is evaluated at compile time
            // and a non-const expression (`env::predecessor_account_id()`, `vec![..]`,
            // `.parse()`) is a compile error at the expression (E0015). Re-evaluating it on
            // every decode of an old record is then harmless: it is the same value every time.
            // It sits in an `impl` with no generated binding, so it resolves like code the user
            // writes there (`Self::X` is the record's). An associated const is evaluated only
            // where it is used, so a free `const _` uses it: a failing `expr` (a panicking
            // `const fn`, an index out of bounds) is E0080 at `cargo check`, used or not.
            let c = format_ident!("__NEAR_KEY_DEFAULT_{}", t, span = d.span());
            defaults.push(quote_spanned! {d.span()=>
                #[doc(hidden)]
                const #c: #ty = #d;
            });
            defaults_forced.push(quote_spanned! {d.span()=>
                const _: () = {
                    let _ = ::core::mem::ManuallyDrop::new(<#name>::#c);
                };
            });
            quote_spanned! {span=> #p::field_or::<#ty>(#l_f, #t, #l_path, || Self::#c) }
        } else if let Some(fp) = default_fn {
            // `default_fn = path`: the escape hatch from the const rule above, for a type
            // with no const constructor (`AccountId`, `Vec`). Coerced to `fn() -> T`, so
            // a closure, a method needing arguments or a wrong return type is an error at the
            // path. Called on EVERY decode where the tag is absent, until some other change
            // rewrites the record (the flush writes the tag then); it must be pure.
            let dfn = local("default_fn");
            let c = quote_spanned! {fp.span()=> let #dfn: fn() -> #ty = #fp; };
            quote_spanned! {span=> #p::field_or::<#ty>(#l_f, #t, #l_path, || { #c #dfn() }) }
        } else {
            quote_spanned! {span=> #p::field::<#ty>(#l_f, #t, #l_path) }
        };
        dec.push(quote_spanned! {span=> #ident: #decode?, });
        if *legacy {
            // A root-only v1 field, stored inline as its Borsh bytes (its prefix), outside
            // the `Value` trait. Owns nothing the v2 cache knows about: no subpath, no binding.
            enc.push(quote! { #p::encode_legacy_field(&mut #l_w, #t, &self.#ident); });
            known_arms.push(quote! { #t => false, });
            unknown_arms.push(quote! { #t => ::core::option::Option::None, });
            continue;
        }
        enc.push(quote! { #p::encode_field(&mut #l_w, #l_of.as_ref(), #t, &self.#ident); });
        known_arms.push(quote! { #t => !<#ty as #s::Value>::INLINE, });
        // a declared field: a nested record checks its own bytes, anything else is known
        unknown_arms.push(quote! { #t => #p::declared_field_collection::<#ty>(#l_w, #l_b), });
        if !is_root {
            // A3 first-write rule: a headerless collection has no first-write signal,
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
    let bind = quote! { #(#s::Value::bind(&mut self.#ident, &#l_path.field(#tag));)* };
    let pending = quote! { #(#s::Value::__write_pending(&self.#ident, &#l_path.field(#tag));)* };

    // `max(.. max(max(0, a), b) .., z)`: no binding for a field type to meet
    let max_subpath = ty.iter().zip(&tag).fold(quote!(0usize), |m, (ty, tag)| {
        quote! { #p::max(#m, #p::field_subpath::<#ty>(#tag)) }
    });
    // A root never leaves kind markers (it is never removed); a record does.
    let writer = if is_root { quote!(#p::RecordWriter::root()) } else { quote!(#p::RecordWriter::new()) };
    let encode_body = quote! {
        let #l_of = #p::parse_orig(#l_orig);
        let mut #l_w = #writer;
        #(#enc)*
        // keep a stored tag iff the code does not know it inline and it is not reserved
        #l_w.finish(#l_orig, |#l_t| match #l_t {
            #(#known_arms)*
            _ => true,
        })
    };
    // The one place each field's decode (and its `FieldDefault` bound) is spelled, shared by
    // `decode_value` and the generated `Default`, so a missing `Default` is reported once.
    let decode_fields = quote! {
        impl #name {
            #(#defaults)*
            #[doc(hidden)]
            #[allow(dead_code)]
            fn __near_decode_fields(
                #l_f: &#p::Fields<'_>,
                #l_path: ::core::option::Option<&#s::Path>,
            ) -> ::core::result::Result<Self, #s::RecordError> {
                ::core::result::Result::Ok(Self { #(#dec)* })
            }
        }
        #(#defaults_forced)*
    };
    let decode_body = quote! {
        let #l_f = #p::parse(#l_bytes)?;
        Self::__near_decode_fields(&#l_f, ::core::option::Option::Some(#l_path))
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
                        let #l_path = <Self as #s::Root>::root_path();
                        <Self as #s::Root>::__decode_root(&[], &#l_path).unwrap_or_else(|#l_e| {
                            ::near_sdk::env::panic_str(&::std::format!(
                                "state: `Default` for a v2 root decodes an empty record: {}",
                                #l_e
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
                    fn __encode_root(&self, #l_orig: ::core::option::Option<&[u8]>) -> ::std::vec::Vec<u8> {
                        #encode_body
                    }
                    fn __decode_root(#l_bytes: &[u8], #l_path: &#s::Path)
                        -> ::core::result::Result<Self, #s::RecordError>
                    {
                        #decode_body
                    }
                    fn __bind_root(&mut self, #l_path: &#s::Path) {
                        #bind
                    }
                    fn __write_pending_root(&self, #l_path: &#s::Path) {
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
                    match Self::__near_decode_fields(&::std::vec::Vec::new(), ::core::option::Option::None) {
                        ::core::result::Result::Ok(#l_v) => #l_v,
                        ::core::result::Result::Err(#l_e) => ::core::panic!("state: {}", #l_e),
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

    // Not reserved names: the view's phantom field differs from every field, the `ViewField`
    // impls' parent parameter from the record's name.
    let marker = unused_ident("__m", fields.iter().map(|f| &f.ident));
    let parent = unused_ident("__R", [name]);

    Ok(quote! {
        #struct_attrs
        #item

        #[doc = #view_doc]
        #[allow(dead_code)]
        #vis struct #view<'a> {
            #(#fvis #ident: <#ty as #s::ViewField<'a, #name>>::Out,)*
            #[doc(hidden)]
            pub #marker: ::core::marker::PhantomData<&'a ()>,
        }

        #[doc = #view_mut_doc]
        #[allow(dead_code)]
        #vis struct #view_mut<'a> {
            #(#fvis #ident: <#ty as #s::ViewFieldMut<'a, #name>>::Out,)*
            #[doc(hidden)]
            pub #marker: ::core::marker::PhantomData<&'a mut ()>,
        }

        const _: () = {
            impl #s::Value for #name {
                const MAX_SUBPATH: usize = #max_subpath;
                const __RECORD: bool = true;
                fn __unknown_collection(#l_bytes: &[u8])
                    -> ::core::option::Option<(::std::vec::Vec<u32>, u8)>
                {
                    let #l_f = #p::parse(#l_bytes).ok()?;
                    for &(#l_t, #l_w, #l_b) in #l_f.iter() {
                        let #l_hit = match #l_t {
                            #(#unknown_arms)*
                            _ => #p::unknown_field_collection(#l_w, #l_b),
                        };
                        // exhaustive, so a binding captured as a constant pattern is E0004,
                        // never a silently skipped hit
                        match #l_hit {
                            ::core::option::Option::Some((mut #l_trail, #l_k)) => {
                                #l_trail.insert(0, #l_t);
                                return ::core::option::Option::Some((#l_trail, #l_k));
                            }
                            ::core::option::Option::None => {}
                        }
                    }
                    ::core::option::Option::None
                }
                fn encode_value(&self, #l_orig: ::core::option::Option<&[u8]>) -> ::std::vec::Vec<u8> {
                    #encode_body
                }
                fn decode_value(#l_bytes: &[u8], #l_path: &#s::Path)
                    -> ::core::result::Result<Self, #s::RecordError>
                {
                    #decode_body
                }
                fn bind(&mut self, #l_path: &#s::Path) {
                    #bind
                }
                fn detach(&mut self) {
                    #(#s::Value::detach(&mut self.#ident);)*
                }
                fn __write_pending(&self, #l_path: &#s::Path) {
                    #pending
                }
            }

            impl #s::Record for #name {
                type View<'a> = #view<'a>;
                type ViewMut<'a> = #view_mut<'a>;
                fn __view<'a>(#l_path: #s::Path) -> #view<'a> {
                    #view {
                        #(#ident: <#ty as #s::ViewField<'a, #name>>::__view_field(#l_path.field(#tag)),)*
                        #marker: ::core::marker::PhantomData
                    }
                }
                #[allow(unused_variables)]
                fn __view_mut_in<'a>(#l_path: #s::Path, #l_anchor: usize) -> #view_mut<'a> {
                    #view_mut {
                        #(#ident: <#ty as #s::ViewFieldMut<'a, #name>>::__view_field_mut(
                            #l_path.field(#tag),
                            #l_anchor
                        ),)*
                        #marker: ::core::marker::PhantomData
                    }
                }
            }

            impl<'a, #parent: #s::Value> #s::ViewField<'a, #parent> for #name {
                type Out = #view<'a>;
                fn __view_field(#l_path: #s::Path) -> #view<'a> {
                    <#name as #s::Record>::__view(#l_path)
                }
            }

            impl<'a, #parent: #s::Value> #s::ViewFieldMut<'a, #parent> for #name {
                type Out = #view_mut<'a>;
                fn __view_field_mut(#l_path: #s::Path, #l_anchor: usize) -> #view_mut<'a> {
                    // a nested record field: its collections live in the enclosing stored record
                    <#name as #s::Record>::__view_mut_in(#l_path, #l_anchor)
                }
            }

            #[allow(unused_lifetimes)]
            impl #s::Removable for #name where #(for<'__x> #ty: #s::Removable,)* {
                fn clear_children(#l_path: &#s::Path) {
                    #(<#ty as #s::Removable>::clear_children(&#l_path.field(#tag));)*
                }
                fn clear_children_some(#l_path: &#s::Path, #l_budget: &mut u32) -> bool {
                    #(if !<#ty as #s::Removable>::clear_children_some(&#l_path.field(#tag), #l_budget) {
                        return false;
                    })*
                    true
                }
            }

            // A record owns no storage iff every field is a `Leaf`; plain `insert` (which
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
pub fn contract_state_v2(ident: &syn::Ident) -> TokenStream {
    quote! {
        const _: () = {
            impl ::near_sdk::state::ContractState for #ident {
                #[inline]
                fn state_key() -> &'static [u8] {
                    // the root byte is spelled once, in `impl Root`
                    const { &[<Self as ::near_sdk::state::Root>::ROOT_BYTE] }
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
