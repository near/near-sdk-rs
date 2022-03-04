use syn::ItemStruct;

pub fn generate_sim_proxy_struct(input: &ItemStruct) -> proc_macro2::TokenStream {
    use quote::{format_ident, quote};
    let ident = &input.ident;
    let new_name = format_ident!("{}Contract", ident);
    let name = quote! {#new_name};
    quote! {
         #[cfg(not(target_arch = "wasm32"))]
         pub struct #name {
            pub account_id: near_sdk::AccountId,
          }
    }
}

pub fn generate_ext_struct(input: &ItemStruct) -> proc_macro2::TokenStream {
    use quote::{format_ident, quote};
    let ident = &input.ident;
    let new_name = format_ident!("{}Ext", ident);
    let name = quote! {#new_name};
    let generics = &input.generics;
    quote! {
      #[must_use]
      pub struct #name {
        pub(crate) account_id: near_sdk::AccountId,
        pub(crate) amount: u128,
        pub(crate) static_gas: near_sdk::Gas,
        pub(crate) gas_weight: u64,
      }

      impl #name {
        pub fn with_amount(mut self, amount: u128) -> Self {
          self.amount = amount;
          self
        }
        pub fn with_static_gas(mut self, static_gas: near_sdk::Gas) -> Self {
          self.static_gas = static_gas;
          self
        }
        pub fn with_unused_gas_weight(mut self, gas_weight: u64) -> Self {
          self.gas_weight = gas_weight;
          self
        }
      }

      impl#generics #ident#generics {
        /// API for calling this contract's functions in a subsequent execution.
        pub fn ext(account_id: near_sdk::AccountId) -> #name {
          #name {
            account_id,
            amount: 0,
            static_gas: near_sdk::Gas(0),
            gas_weight: 1,
          }
        }
      }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn ext_gen() {
        let st: ItemStruct = syn::parse_str("struct Test { a: u8 }").unwrap();
        let actual = generate_ext_struct(&st);
        let expected = quote!(
          #[must_use]
          pub struct TestExt {
            pub(crate) account_id: near_sdk::AccountId,
            pub(crate) amount: u128,
            pub(crate) static_gas: near_sdk::Gas,
            pub(crate) gas_weight: u64,
          }
          impl TestExt {
            pub fn with_amount(mut self, amount: u128) -> Self {
              self.amount = amount;
              self
            }
            pub fn with_static_gas(mut self, static_gas: near_sdk::Gas) -> Self {
              self.static_gas = static_gas;
              self
            }
            pub fn with_unused_gas_weight(mut self, gas_weight: u64) -> Self {
              self.gas_weight = gas_weight;
              self
            }
          }
          impl Test {
            /// API for calling this contract's functions in a subsequent execution.
            pub fn ext(account_id: near_sdk::AccountId) -> TestExt {
              TestExt {
                account_id,
                amount: 0,
                static_gas: near_sdk::Gas(0),
                gas_weight: 1,
              }
            }
          }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }
}
