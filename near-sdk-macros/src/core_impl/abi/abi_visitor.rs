use super::TypeRegistry;
use crate::ItemImplInfo;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::visit::Visit;
use syn::{Error, ItemImpl};

/// Information relevant to ABI extracted from the `impl` section decorated with `#[near_bindgen]`.
#[derive(Default)]
pub struct AbiVisitor {
    impl_item_infos: Vec<ItemImplInfo>,
    /// Errors that occured while extracting the data.
    errors: Vec<Error>,
}

impl<'ast> Visit<'ast> for AbiVisitor {
    fn visit_item_impl(&mut self, i: &'ast ItemImpl) {
        let has_near_sdk_attr = i
            .attrs
            .iter()
            .any(|attr| attr.path.to_token_stream().to_string().as_str() == "near_bindgen");
        if has_near_sdk_attr {
            match ItemImplInfo::new(&mut i.clone()) {
                Ok(info) => self.impl_item_infos.push(info),
                Err(err) => self.errors.push(err),
            }
        }
        syn::visit::visit_item_impl(self, i);
    }
}
impl AbiVisitor {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn generate_abi_function(&self) -> syn::Result<TokenStream2> {
        let mut registry = TypeRegistry::new();
        if !self.errors.is_empty() {
            return Err(self.errors[0].clone());
        }
        let functions: Vec<TokenStream2> = self
            .impl_item_infos
            .iter()
            .flat_map(|i| &i.methods)
            .map(|m| m.abi_struct(&mut registry))
            .collect();
        let types: Vec<TokenStream2> = registry
            .types
            .iter()
            .map(|(t, id)| {
                quote! {
                    near_sdk::__private::AbiTypeDef { id: #id, schema: gen.subschema_for::<#t>() }
                }
            })
            .collect();
        Ok(quote! {
            const _: () = {
                #[no_mangle]
                #[cfg(not(target_arch = "wasm32"))]
                pub fn __near_abi() -> near_sdk::__private::AbiRoot {
                    use borsh::*;
                    let mut gen = schemars::gen::SchemaGenerator::default();
                    let types = vec![#(#types),*];
                    near_sdk::__private::AbiRoot::new(
                        near_sdk::__private::Abi {
                            functions: vec![#(#functions),*],
                            types: types,
                            root_schema: gen.into_root_schema_for::<String>(),
                        }
                    )
                }
            };
        })
    }
}
