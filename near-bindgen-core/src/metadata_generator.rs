use crate::{BindgenArgType, ImplItemMethodInfo};
use syn::{Receiver, ReturnType};

use std::borrow::Borrow;
use std::cell::RefCell;
use syn::export::TokenStream2;

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
            quote! {
                Some(Input::schema_container())
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
                 callbacks: #callbacks,
                 callbacks_vec: #callbacks_vec,
                 result: #result
             }
            });
        });
    }
}

/// Produce method that exposes metadata.
pub fn generate_metadata_method() -> TokenStream2 {
    let methods = (*METADATA.borrow()).clone();
    quote! {
        #[cfg(target_arch = "wasm32")]
        #[no_mangle]
        pub extern "C" fn metadata() {
            let metadata = near_bindgen::Metadata::new(vec![
                #(#methods),*
            ]);
            let data = borsh::try_to_vec_with_schema(&metadata).expect("Failed to serialize the metadata using Borsh");
            near_bindgen::env::value_return(&data);
        }
    }
}
