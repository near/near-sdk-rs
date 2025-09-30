use crate::core_impl::{serializer, AttrSigInfo};
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{format_ident, quote, ToTokens};
use syn::{parse_quote, Attribute, Generics, Path, Signature};

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
        pub fn ext(account_id: ::near_sdk::AccountId) -> #name {
            #name {
                account_id,
                deposit: ::near_sdk::NearToken::from_near(0),
                static_gas: ::near_sdk::Gas::from_gas(0),
                gas_weight: ::near_sdk::GasWeight::default(),
            }
        }
    };
    if let Some(generics) = generic_details {
        // If ext generation is on struct, make ext function associated with struct not module
        ext_code = quote! {
            impl #generics #ident #generics {
                #ext_code
            }
        };
    }

    quote! {
      #[must_use]
      pub struct #name {
          pub(crate) account_id: ::near_sdk::AccountId,
          pub(crate) deposit: ::near_sdk::NearToken,
          pub(crate) static_gas: ::near_sdk::Gas,
          pub(crate) gas_weight: ::near_sdk::GasWeight,
      }

      impl #name {
          pub fn with_attached_deposit(mut self, amount: ::near_sdk::NearToken) -> Self {
              self.deposit = amount;
              self
          }
          pub fn with_static_gas(mut self, static_gas: ::near_sdk::Gas) -> Self {
              self.static_gas = static_gas;
              self
          }
          pub fn with_unused_gas_weight(mut self, gas_weight: u64) -> Self {
              self.gas_weight = ::near_sdk::GasWeight(gas_weight);
              self
          }
      }

      #ext_code
    }
}

/// Non-bindgen attributes on contract methods should not be forwarded to the
/// corresponding `_Ext` methods by default. It may lead to compilation errors
/// or unexpected behavior. For a more detailed motivation, see [#959].
///
/// However, some attributes should be forwarded and they are defined here.
///
/// [#959]: https://github.com/near/near-sdk-rs/pull/959
const FN_ATTRIBUTES_TO_FORWARD: [&str; 1] = [
    // Allow some contract methods to be feature gated, for example:
    //
    // ```
    // impl Contract {
    //     #[cfg(integration_tests)]
    //     pub fn test_method(&mut self) { /* ... */ }
    // }
    // ```
    //
    // In that scenario `ContractExt::test_method` should be included only if
    // `integration_tests` is enabled.
    "cfg",
];

/// Returns whether `attribute` should be forwarded to `_Ext` methods, see
/// [`FN_ATTRIBUTES_TO_FORWARD`].
fn is_fn_attribute_to_forward(attribute: &Attribute) -> bool {
    for to_forward in FN_ATTRIBUTES_TO_FORWARD.iter() {
        let to_forward_ident = Ident::new(to_forward, Span::mixed_site());
        let to_forward_path: Path = parse_quote! { #to_forward_ident };
        if &to_forward_path == attribute.meta.path() {
            return true;
        }
    }
    false
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
        if is_fn_attribute_to_forward(attribute) {
            attribute.to_tokens(&mut new_non_bindgen_attrs);
        }
    }
    let Signature { generics, .. } = original_sig;
    quote! {
        #new_non_bindgen_attrs
        pub fn #ident #generics(self, #pat_type_list) -> ::near_sdk::Promise {
            let __args = #serialize;
            ::near_sdk::Promise::new(self.account_id)
            .function_call_weight(
                ::std::string::String::from(#ident_str),
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
    use syn::{parse_quote, ImplItemFn, ItemStruct, Type};
    use crate::core_impl::utils::test_helpers::{local_insta_assert_snapshot, pretty_print_syn_str};

    #[test]
    fn ext_gen() {
        let st: ItemStruct = parse_quote! { struct Test { a: u8 } };
        let actual = generate_ext_structs(&st.ident, Some(&st.generics));
       
        local_insta_assert_snapshot!(pretty_print_syn_str(&actual).unwrap());
    }

    #[test]
    fn module_ext_gen() {
        let ident: Ident = parse_quote! { Test };
        let actual = generate_ext_structs(&ident, None);
    
        local_insta_assert_snapshot!(pretty_print_syn_str(&actual).unwrap());
    }

    /// Verifies that only whitelisted attributes are forwarded to `_Ext`
    /// methods.
    #[test]
    fn ext_fn_non_bindgen_attrs() {
        let impl_type: Type = parse_quote! { Hello };
        let mut method: ImplItemFn = parse_quote! {
            #[cfg(target_os = "linux")]
            #[inline]
            #[warn(unused)]
            pub fn method(&self) { }
        };
        let method_info = ImplItemMethodInfo::new(&mut method, None, impl_type).unwrap().unwrap();
        let actual = generate_ext_function(&method_info.attr_signature_info);

        local_insta_assert_snapshot!(pretty_print_syn_str(&actual).unwrap());
    }

    #[test]
    fn ext_basic_json() {
        let impl_type: Type = parse_quote! { Hello };
        let mut method: ImplItemFn = parse_quote! {
            pub fn method(&self, k: &String) { }
        };
        let method_info = ImplItemMethodInfo::new(&mut method, None, impl_type).unwrap().unwrap();
        let actual = generate_ext_function(&method_info.attr_signature_info);
     
        local_insta_assert_snapshot!(pretty_print_syn_str(&actual).unwrap());
    }

    #[test]
    fn ext_basic_borsh() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: syn::ImplItemFn = parse_quote! {
          pub fn borsh_test(&mut self, #[serializer(borsh)] a: String) {}
        };
        let method_info = ImplItemMethodInfo::new(&mut method, None, impl_type).unwrap().unwrap();
        let actual = generate_ext_function(&method_info.attr_signature_info);
       
        local_insta_assert_snapshot!(pretty_print_syn_str(&actual).unwrap());
    }
}
