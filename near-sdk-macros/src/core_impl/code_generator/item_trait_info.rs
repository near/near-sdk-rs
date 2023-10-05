use crate::core_impl::ext::{generate_ext_function_wrappers, generate_ext_structs};
use crate::core_impl::info_extractor::ItemTraitInfo;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

impl ItemTraitInfo {
    /// Generate code that wraps external calls.
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
    use crate::core_impl::utils::test_helpers::{local_insta_assert_snapshot, pretty_print_syn_str};

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
        local_insta_assert_snapshot!(pretty_print_syn_str(&actual).unwrap());
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

        local_insta_assert_snapshot!(pretty_print_syn_str(&actual).unwrap());
    }
}
