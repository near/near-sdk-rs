use crate::ImplItemMethodInfo;
use syn::spanned::Spanned;
use syn::{Error, ImplItem, ItemImpl, Type};

/// Information extracted from `impl` section.
pub struct ItemImplInfo {
    /// The type for which this `impl` is written.
    pub ty: Type,
    /// Info extracted for each public method.
    pub methods: Vec<ImplItemMethodInfo>,
}

impl ItemImplInfo {
    pub fn new(original: &mut ItemImpl) -> syn::Result<Self> {
        if !original.generics.params.is_empty() {
            return Err(Error::new(
                original.generics.params.span(),
                "Impl type parameters are not supported for smart contracts.",
            ));
        }
        let ty = (*original.self_ty.as_ref()).clone();
        let trait_ = original.trait_.as_ref().map(|(_not, path, _for)| path);

        let mut methods = vec![];
        let mut errors = vec![];
        for subitem in &mut original.items {
            if let ImplItem::Fn(m) = subitem {
                match ImplItemMethodInfo::new(m, trait_.cloned(), ty.clone()) {
                    Ok(Some(method_info)) => methods.push(method_info),
                    Ok(None) => {} // do nothing
                    Err(e) => errors.push(e),
                }
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

        Ok(Self { ty, methods })
    }
}
