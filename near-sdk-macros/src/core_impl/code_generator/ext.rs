use crate::core_impl::{serializer, AttrSigInfo};
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::{format_ident, quote, ToTokens};
use syn::{Generics, Signature};

/// Generates inner ext code for structs and modules. If intended for a struct, generic details
/// for the struct should be passed in through `generic_details` and the `ext` method will be
/// added as an impl to the struct ident.
pub(crate) fn generate_ext_structs(
    ident: &Ident,
    generic_details: Option<&Generics>,
) -> proc_macro2::TokenStream {
    let name = format_ident!("{}Ext", ident);
    let mut ext_code = quote! {
        /// API for calling this contract's functions in a subsequent execution.
        pub fn ext(account_id: near_sdk::AccountId) -> #name {
            #name {
                account_id,
                deposit: 0,
                static_gas: near_sdk::Gas(0),
                gas_weight: near_sdk::GasWeight::default(),
            }
        }
    };
    if let Some(generics) = generic_details {
        // If ext generation is on struct, make ext function associated with struct not module
        ext_code = quote! {
            impl#generics #ident#generics {
                #ext_code
            }
        };
    }

    quote! {
      #[must_use]
      pub struct #name {
          pub(crate) account_id: near_sdk::AccountId,
          pub(crate) deposit: near_sdk::Balance,
          pub(crate) static_gas: near_sdk::Gas,
          pub(crate) gas_weight: near_sdk::GasWeight,
      }

      impl #name {
          pub fn with_attached_deposit(mut self, amount: near_sdk::Balance) -> Self {
              self.deposit = amount;
              self
          }
          pub fn with_static_gas(mut self, static_gas: near_sdk::Gas) -> Self {
              self.static_gas = static_gas;
              self
          }
          pub fn with_unused_gas_weight(mut self, gas_weight: u64) -> Self {
              self.gas_weight = near_sdk::GasWeight(gas_weight);
              self
          }
      }

      #ext_code
    }
}

/// Generate methods on <StructName>Ext to enable calling each method.
pub(crate) fn generate_ext_function_wrappers<'a>(
    ident: &Ident,
    methods: impl IntoIterator<Item = &'a AttrSigInfo>,
) -> TokenStream2 {
    let ext_ident = format_ident!("{}Ext", ident);
    let mut res = TokenStream2::new();
    for method in methods {
        res.extend(generate_ext_function(method));
    }
    quote! {
        impl #ext_ident {
            #res
        }
    }
}

fn generate_ext_function(attr_signature_info: &AttrSigInfo) -> TokenStream2 {
    let pat_type_list = attr_signature_info.pat_type_list();
    let serialize =
        serializer::generate_serializer(attr_signature_info, &attr_signature_info.input_serializer);

    let AttrSigInfo { non_bindgen_attrs, ident, original_sig, .. } = attr_signature_info;
    let ident_str = ident.to_string();
    let mut new_non_bindgen_attrs = TokenStream2::new();
    for attribute in non_bindgen_attrs.iter() {
        attribute.to_tokens(&mut new_non_bindgen_attrs);
    }
    let Signature { generics, .. } = original_sig;
    quote! {
        #new_non_bindgen_attrs
        pub fn #ident#generics(self, #pat_type_list) -> near_sdk::Promise {
            let __args = #serialize;
            near_sdk::Promise::new(self.account_id)
            .function_call_weight(
                #ident_str.to_string(),
                __args,
                self.deposit,
                self.static_gas,
                self.gas_weight,
            )
        }
    }
}

#[rustfmt::skip]
#[cfg(test)]
mod tests {
    use crate::core_impl::ImplItemMethodInfo;

    use super::*;
    use quote::quote;
    use syn::{parse_quote, ImplItemMethod, ItemStruct, Type};

    #[test]
    fn ext_gen() {
        let st: ItemStruct = parse_quote! { struct Test { a: u8 } };
        let actual = generate_ext_structs(&st.ident, Some(&st.generics));
        let expected = quote!(
          #[must_use]
          pub struct TestExt {
              pub(crate) account_id: near_sdk::AccountId,
              pub(crate) deposit: near_sdk::Balance,
              pub(crate) static_gas: near_sdk::Gas,
              pub(crate) gas_weight: near_sdk::GasWeight,
          }
          impl TestExt {
              pub fn with_attached_deposit(mut self, amount: near_sdk::Balance) -> Self {
                  self.deposit = amount;
                  self
              }
              pub fn with_static_gas(mut self, static_gas: near_sdk::Gas) -> Self {
                  self.static_gas = static_gas;
                  self
              }
              pub fn with_unused_gas_weight(mut self, gas_weight: u64) -> Self {
                  self.gas_weight = near_sdk::GasWeight(gas_weight);
                  self
              }
          }
          impl Test {
            /// API for calling this contract's functions in a subsequent execution.
            pub fn ext(account_id: near_sdk::AccountId) -> TestExt {
                TestExt {
                    account_id,
                    deposit: 0,
                    static_gas: near_sdk::Gas(0),
                    gas_weight: near_sdk::GasWeight::default(),
                }
            }
          }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn module_ext_gen() {
        let ident: Ident = parse_quote! { Test };
        let actual = generate_ext_structs(&ident, None);
        let expected = quote!(
          #[must_use]
          pub struct TestExt {
              pub(crate) account_id: near_sdk::AccountId,
              pub(crate) deposit: near_sdk::Balance,
              pub(crate) static_gas: near_sdk::Gas,
              pub(crate) gas_weight: near_sdk::GasWeight,
          }
          impl TestExt {
              pub fn with_attached_deposit(mut self, amount: near_sdk::Balance) -> Self {
                  self.deposit = amount;
                  self
              }
              pub fn with_static_gas(mut self, static_gas: near_sdk::Gas) -> Self {
                  self.static_gas = static_gas;
                  self
              }
              pub fn with_unused_gas_weight(mut self, gas_weight: u64) -> Self {
                  self.gas_weight = near_sdk::GasWeight(gas_weight);
                  self
              }
          }
          /// API for calling this contract's functions in a subsequent execution.
          pub fn ext(account_id: near_sdk::AccountId) -> TestExt {
              TestExt {
                  account_id,
                  deposit: 0,
                  static_gas: near_sdk::Gas(0),
                  gas_weight: near_sdk::GasWeight::default(),
              }
          }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn ext_basic_json() {
        let impl_type: Type = parse_quote! { Hello };
        let mut method: ImplItemMethod = parse_quote! {
            pub fn method(&self, k: &String) { }
        };
        let method_info = ImplItemMethodInfo::new(&mut method, impl_type).unwrap();
        let actual = generate_ext_function(&method_info.attr_signature_info);
        let expected = quote!(
            pub fn method(self, k: &String,) -> near_sdk::Promise {
                let __args = {#[derive(near_sdk :: serde :: Serialize)]
                    #[serde(crate = "near_sdk::serde")]
                    struct Input<'nearinput> {
                        k: &'nearinput String,
                    }
                    let __args = Input { k: &k, };
                    near_sdk::serde_json::to_vec(&__args)
                        .expect("Failed to serialize the cross contract args using JSON.")
                };
                near_sdk::Promise::new(self.account_id).function_call_weight(
                    "method".to_string(),
                    __args,
                    self.deposit,
                    self.static_gas,
                    self.gas_weight,
                )
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }

    #[test]
    fn ext_basic_borsh() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod = parse_quote! {
          pub fn borsh_test(&mut self, #[serializer(borsh)] a: String) {}
        };
        let method_info = ImplItemMethodInfo::new(&mut method, impl_type).unwrap();
        let actual = generate_ext_function(&method_info.attr_signature_info);
        let expected = quote!(
          pub fn borsh_test(self, a: String,) -> near_sdk::Promise {
            let __args = {
              #[derive(near_sdk :: borsh :: BorshSerialize)]
              struct Input<'nearinput> {
                  a: &'nearinput String,
              }
              let __args = Input { a: &a, };
              near_sdk::borsh::BorshSerialize::try_to_vec(&__args)
                  .expect("Failed to serialize the cross contract args using Borsh.")
            };
              near_sdk::Promise::new(self.account_id)
                  .function_call_weight(
                      "borsh_test".to_string(),
                      __args,
                      self.deposit,
                      self.static_gas,
                      self.gas_weight,
                  )
          }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }
}
