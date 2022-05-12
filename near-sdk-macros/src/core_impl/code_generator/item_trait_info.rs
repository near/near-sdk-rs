use crate::core_impl::ext::{generate_ext_function_wrappers, generate_ext_structs};
use crate::core_impl::info_extractor::ItemTraitInfo;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

impl ItemTraitInfo {
    /// Generate code that wrapps external calls.
    pub fn wrap_trait_ext(&self) -> TokenStream2 {
        let mod_name = &self.mod_name;
        let ext_structs = generate_ext_structs(&self.original.ident, None);

        let ext_methods = generate_ext_function_wrappers(
            &self.original.ident,
            self.methods.iter().map(|m| &m.attr_sig_info),
        );

        quote! {
            pub mod #mod_name {
                use super::*;
                #ext_structs
                #ext_methods
            }
        }
    }
}

// Rustfmt removes comas.
#[rustfmt::skip]
#[cfg(test)]
mod tests {
    use syn::ItemTrait;
    use quote::quote;
    use crate::core_impl::info_extractor::ItemTraitInfo;

    #[test]
    fn ext_basic() {
        let mut t: ItemTrait = syn::parse2(
            quote!{
                pub trait ExternalCrossContract {
                    fn merge_sort(&self, arr: Vec<u8>) -> PromiseOrValue<Vec<u8>>;
                    fn merge(
                        &self,
                        #[callback_unwrap]
                        #[serializer(borsh)]
                        data0: Vec<u8>,
                        #[callback_unwrap]
                        #[serializer(borsh)]
                        data1: Vec<u8>,
                    ) -> Vec<u8>;
                }
            }
        ).unwrap();
        let info = ItemTraitInfo::new(&mut t, None).unwrap();
        let actual = info.wrap_trait_ext();

        let expected = quote! {
            pub mod external_cross_contract {
                use super::*;
                #[must_use]
                pub struct ExternalCrossContractExt {
                    pub(crate) account_id: near_sdk::AccountId,
                    pub(crate) deposit: near_sdk::Balance,
                    pub(crate) static_gas: near_sdk::Gas,
                    pub(crate) gas_weight: near_sdk::GasWeight,
                }
                impl ExternalCrossContractExt {
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
                pub fn ext(account_id: near_sdk::AccountId) -> ExternalCrossContractExt {
                    ExternalCrossContractExt {
                        account_id,
                        deposit: 0,
                        static_gas: near_sdk::Gas(0),
                        gas_weight: near_sdk::GasWeight::default(),
                    }
                }
                impl ExternalCrossContractExt {
                    pub fn merge_sort(
                        self,
                        arr: Vec<u8>,
                    ) -> near_sdk::Promise {
                        let __args = {
                            #[derive(near_sdk :: serde :: Serialize)]
                            #[serde(crate = "near_sdk::serde")]
                            struct Input<'nearinput> {
                                arr: &'nearinput Vec<u8>,
                            }
                            let __args = Input { arr: &arr, };
                            near_sdk::serde_json::to_vec(&__args)
                                .expect("Failed to serialize the cross contract args using JSON.")
                        };
                        near_sdk::Promise::new(self.account_id)
                            .function_call_weight(
                                "merge_sort".to_string(),
                                __args,
                                self.deposit,
                                self.static_gas,
                                self.gas_weight,
                            )
                    }
                    pub fn merge(self,) -> near_sdk::Promise {
                        let __args = vec![];
                        near_sdk::Promise::new(self.account_id)
                            .function_call_weight(
                                "merge".to_string(),
                                __args,
                                self.deposit,
                                self.static_gas,
                                self.gas_weight,
                            )
                    }
                }
            }
        };
        assert_eq!(actual.to_string(), expected.to_string());
    }

    #[test]
    fn serialize_with_borsh() {
        let mut t: ItemTrait = syn::parse2(
            quote!{
              trait Test {
                #[result_serializer(borsh)]
                fn test(#[serializer(borsh)] v: Vec<String>) -> Vec<String>;
              }
            }
        ).unwrap();
        let info = ItemTraitInfo::new(&mut t, None).unwrap();
        let actual = info.wrap_trait_ext();

        let expected = quote! {
          pub mod test {
            use super::*;
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
            impl TestExt {
                pub fn test(
                    self,
                    v: Vec<String>,
                ) -> near_sdk::Promise {
                    let __args = {
                        #[derive(near_sdk :: borsh :: BorshSerialize)]
                        struct Input<'nearinput> {
                            v: &'nearinput Vec<String>,
                        }
                        let __args = Input { v: &v, };
                        near_sdk::borsh::BorshSerialize::try_to_vec(&__args)
                            .expect("Failed to serialize the cross contract args using Borsh.")
                    };
                    near_sdk::Promise::new(self.account_id)
                        .function_call_weight(
                            "test".to_string(),
                            __args,
                            self.deposit,
                            self.static_gas,
                            self.gas_weight,
                        )
                }
            }
        }
        };
        assert_eq!(actual.to_string(), expected.to_string());
    }
}
