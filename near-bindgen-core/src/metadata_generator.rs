use crate::{BindgenArgType, ImplItemMethodInfo, SerializerType};

use quote::quote;
use std::cell::RefCell;
use syn::export::TokenStream2;
use syn::ReturnType;

thread_local! {
    static METADATA: RefCell<Vec<TokenStream2>> = RefCell::new(vec![]);
}

impl ImplItemMethodInfo {
    /// Record metadata for this method in a global container.
    pub fn record_metadata(&self) {
        let method_name_str = self.attr_signature_info.ident.to_string();
        let is_view = match &self.attr_signature_info.receiver {
            None => true,
            Some(rec) => rec.mutability.is_none(),
        };
        let is_init = self.attr_signature_info.is_init;
        let args = if self.attr_signature_info.input_args().next().is_some() {
            let input_struct = self.attr_signature_info.input_struct();
            // If input args are JSON then we need to additionally specify schema for them.
            let additional_schema = match &self.attr_signature_info.input_serializer {
                SerializerType::Borsh => TokenStream2::new(),
                SerializerType::JSON => quote! {
                    #[derive(borsh::BorshSchema)]
                },
            };
            quote! {
                {
                    #additional_schema
                    #input_struct
                    Some(Input::schema_container())
                }
            }
        } else {
            quote! {
                 None
            }
        };
        let callbacks: Vec<_> = self
            .attr_signature_info
            .args
            .iter()
            .filter(|arg| match arg.bindgen_ty {
                BindgenArgType::CallbackArg => true,
                _ => false,
            })
            .map(|arg| {
                let ty = &arg.ty;
                quote! {
                    #ty::schema_container()
                }
            })
            .collect();
        let callbacks_vec = match self
            .attr_signature_info
            .args
            .iter()
            .filter(|arg| match arg.bindgen_ty {
                BindgenArgType::CallbackArgVec => true,
                _ => false,
            })
            .last()
        {
            None => {
                quote! {
                    None
                }
            }
            Some(arg) => {
                let ty = &arg.ty;
                quote! {
                    Some(#ty::schema_container())
                }
            }
        };
        let result = match &self.attr_signature_info.returns {
            ReturnType::Default => {
                quote! {
                    None
                }
            }
            ReturnType::Type(_, ty) => {
                quote! {
                    Some(#ty::schema_container())
                }
            }
        };

        METADATA.with(move |m| {
            m.borrow_mut().push(quote! {
             near_bindgen::MethodMetadata {
                 name: #method_name_str.to_string(),
                 is_view: #is_view,
                 is_init: #is_init,
                 args: #args,
                 callbacks: vec![#(#callbacks),*],
                 callbacks_vec: #callbacks_vec,
                 result: #result
             }
            });
        });
    }
}

/// Produce method that exposes metadata.
pub fn generate_metadata_method() -> TokenStream2 {
    let methods: Vec<TokenStream2> = METADATA.with(|m| (*m.borrow()).clone());
    quote! {
        #[cfg(target_arch = "wasm32")]
        #[no_mangle]
        pub extern "C" fn metadata() {
            use borsh::*;
            let metadata = near_bindgen::Metadata::new(vec![
                #(#methods),*
            ]);
            let data = borsh::try_to_vec_with_schema(&metadata).expect("Failed to serialize the metadata using Borsh");
            near_bindgen::env::value_return(&data);
        }
    }
}

#[rustfmt::skip]
#[cfg(test)]
mod tests {
    use syn::{Type, ImplItemMethod};
    use quote::quote;
    use crate::info_extractor::ImplItemMethodInfo;
    use super::*;

    #[test]
    fn several_methods() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod = syn::parse_str("fn f1(&self) { }").unwrap();
        let method_info = ImplItemMethodInfo::new(&mut method, impl_type.clone()).unwrap();
        method_info.record_metadata();

        let mut method: ImplItemMethod = syn::parse_str("fn f2(&mut self, arg0: FancyStruct, arg1: u64) { }").unwrap();
        let method_info = ImplItemMethodInfo::new(&mut method, impl_type.clone()).unwrap();
        method_info.record_metadata();

        let mut method: ImplItemMethod = syn::parse_str("fn f3(&mut self, arg0: FancyStruct, arg1: u64) -> Result<IsOk, Error> { }").unwrap();
        let method_info = ImplItemMethodInfo::new(&mut method, impl_type.clone()).unwrap();
        method_info.record_metadata();

        let actual = generate_metadata_method();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn metadata() {
                use borsh::*;
                let metadata = near_bindgen::Metadata::new(vec![
                    near_bindgen::MethodMetadata {
                        name: "f1".to_string(),
                        is_view: true,
                        is_init: false,
                        args: None,
                        callbacks: vec![],
                        callbacks_vec: None,
                        result: None
                    },
                    near_bindgen::MethodMetadata {
                        name: "f2".to_string(),
                        is_view: false,
                        is_init: false,
                        args: {
                            #[derive(borsh::BorshSchema)]
                            #[derive(serde :: Deserialize, serde :: Serialize)]
                            struct Input {
                                arg0: FancyStruct,
                                arg1: u64,
                            }
                            Some(Input::schema_container())
                        },
                        callbacks: vec![],
                        callbacks_vec: None,
                        result: None
                    },
                    near_bindgen::MethodMetadata {
                        name: "f3".to_string(),
                        is_view: false,
                        is_init: false,
                        args: {
                            #[derive(borsh::BorshSchema)]
                            #[derive(serde :: Deserialize, serde :: Serialize)]
                            struct Input {
                                arg0: FancyStruct,
                                arg1: u64,
                            }
                            Some(Input::schema_container())
                        },
                        callbacks: vec![],
                        callbacks_vec: None,
                        result: Some(Result < IsOk, Error > ::schema_container())
                    }
                ]);
                let data = borsh::try_to_vec_with_schema(&metadata)
                    .expect("Failed to serialize the metadata using Borsh");
                near_bindgen::env::value_return(&data);
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }
}
