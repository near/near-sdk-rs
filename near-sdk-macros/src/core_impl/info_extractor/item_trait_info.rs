use super::TraitItemMethodInfo;
use inflector::Inflector;
use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{Error, Ident, ItemTrait, TraitItem};

/// Information extracted from `ItemTrait`.
pub struct ItemTraitInfo {
    /// The name of the module that will be used to generate the module.
    pub mod_name: Ident,
    /// Information extracted from the methods.
    pub methods: Vec<TraitItemMethodInfo>,
    /// The original AST.
    pub original: ItemTrait,
}

impl ItemTraitInfo {
    pub fn new(original: &mut ItemTrait, mod_name_override: Option<Ident>) -> syn::Result<Self> {
        let mod_name = mod_name_override.unwrap_or({
            let res = original.ident.to_string().to_snake_case();
            Ident::new(&res, original.span())
        });

        let mut methods = vec![];
        let mut errors = vec![];
        for item in &mut original.items {
            match item {
                TraitItem::Type(_) => errors.push(Error::new(
                    item.span(),
                    "Traits for external contracts do not support associated trait types yet.",
                )),
                TraitItem::Fn(method) => {
                    match TraitItemMethodInfo::new(method, &original.ident.to_token_stream()) {
                        Ok(method_info) => methods.push(method_info),
                        Err(e) => errors.push(e),
                    };

                    if method.default.is_some() {
                        errors.push(Error::new(
                            method.span(),
                            "Traits that are used to describe external contract should not include
                             default implementations because this is not a valid use case of traits
                             to describe external contracts.",
                        ));
                    }
                }
                _ => {}
            }
        }
        if !errors.is_empty() {
            // Combine all errors into one
            let combined_error = errors.into_iter().reduce(|mut l, r| {
                l.combine(r);
                l
            });
            return Err(combined_error.unwrap());
        }
        Ok(Self { original: original.clone(), mod_name, methods })
    }
}
