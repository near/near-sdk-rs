use crate::core_impl::ImplItemMethodInfo;
use crate::ItemImplInfo;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
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
        match ItemImplInfo::new(&mut i.clone()) {
            Ok(info) => self.impl_item_infos.push(info),
            Err(err) => self.errors.push(err),
        }
        syn::visit::visit_item_impl(self, i);
    }
}
impl AbiVisitor {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn generate_abi_function(&self) -> syn::Result<TokenStream2> {
        if !self.errors.is_empty() {
            return Err(self.errors[0].clone());
        }

        let public_functions: Vec<&ImplItemMethodInfo> = self
            .impl_item_infos
            .iter()
            .flat_map(|i| {
                i.methods.iter().filter(|m| m.is_public || i.is_trait_impl).collect::<Vec<_>>()
            })
            .collect();
        if public_functions.is_empty() {
            // Short-circuit if there are not public functions to export to ABI
            return Ok(TokenStream2::new());
        }

        let functions: Vec<TokenStream2> =
            public_functions.iter().map(|m| m.abi_struct()).collect();
        let first_function_name = &public_functions[0].attr_signature_info.ident;
        let near_abi_symbol = format_ident!("__near_abi_{}", &first_function_name);
        Ok(quote! {
            #[cfg(not(target_arch = "wasm32"))]
            const _: () = {
                #[no_mangle]
                pub fn #near_abi_symbol() -> near_sdk::__private::AbiRoot {
                    let mut gen = near_sdk::__private::schemars::gen::SchemaGenerator::default();
                    near_sdk::__private::AbiRoot::new(
                        near_sdk::__private::Abi {
                            functions: vec![#(#functions),*],
                            root_schema: gen.into_root_schema_for::<String>(),
                        }
                    )
                }
            };
        })
    }
}
