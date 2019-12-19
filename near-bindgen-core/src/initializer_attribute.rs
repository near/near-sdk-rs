use crate::{arg_parsing, publicly_accessible};
use proc_macro2::Ident;
use syn::export::{TokenStream2, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{Error, FnArg, ImplItemMethod, Token, Type};
use quote::quote;

/// Parses the following syntax of an attribute of an attribute macro.
///
/// # Example:
///
/// Suppose this is the code that declares that the smart contract can be initialized with `new`.
/// ```ignore
/// #[near_bindgen]
/// struct A {
///     a: u64,
///     b: String,
/// }
///
/// #[near_bindgen(init => new)]
/// impl A {
///     pub fn new(a: u64, b: String) -> Self {
///         Self {a, b}
///     }
/// }
/// ```
///
/// What we parse in this module is the following custom syntax: `init => new`.
pub struct InitAttr {
    pub ident: Ident,
}

impl Parse for InitAttr {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        let init_ident: Ident = input.parse()?;
        if init_ident.to_string() != "init".to_string() {
            return Err(Error::new(
                init_ident.span(),
                "Expected syntax: init => <name of the method>",
            ));
        }
        input.parse::<Token![=]>()?;
        input.parse::<Token![>]>()?;
        let ident: Ident = input.parse()?;
        Ok(Self { ident })
    }
}

/// Attempts processing initialization method. Expects method to be static (not take `self`) and
/// return `Self`.
pub fn process_init_method(
    method: &ImplItemMethod,
    impl_type: &Type,
    is_trait_impl: bool,
) -> syn::Result<TokenStream2> {
    let attrs = method.attrs.iter().fold(TokenStream2::new(), |mut acc, attr| {
        let attr_str = attr.path.to_token_stream().to_string();
        if attr_str != "callback_args_vec" && attr_str != "callback_args" {
            attr.to_tokens(&mut acc);
        }
        acc
    });
    if !publicly_accessible(method, is_trait_impl) {
        return Err(Error::new(
            method.sig.generics.params.span(),
            "Initialization method should have public visibility.",
        ));
    }
    if !method.sig.generics.params.is_empty() {
        return Err(Error::new(
            method.sig.generics.params.span(),
            "Initialization method cannot use type parameters.",
        ));
    }

    let (arg_parsing_code, arg_list) = arg_parsing::get_arg_parsing(method)?;

    for arg in &method.sig.inputs {
        match arg {
            FnArg::Receiver(_) => {
                return Err(Error::new(method.span(), "Initialization method cannot take `self`."));
            }
            _ => {}
        }
    }
    let env_creation = quote! {
        near_bindgen::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
    };
    let method_name = &method.sig.ident;
    let method_invocation = quote! {
        let contract = #impl_type::#method_name(#arg_list);
    };
    let state_ser_code = quote! {
        near_bindgen::env::state_write(&contract);
    };
    let method_body = quote! {
        #env_creation
        #arg_parsing_code
        #method_invocation
        #state_ser_code
    };

    Ok(quote! {
        #attrs
        #[cfg(target_arch = "wasm32")]
        #[no_mangle]
        pub extern "C" fn #method_name() {
            #method_body
        }
    })
}

// Rustfmt removes comas.
#[rustfmt::skip]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_arg_parsing() {
        let init_attr: InitAttr = syn::parse_str("init => new").unwrap();
        assert_eq!(init_attr.ident.to_string(), "new".to_string());
    }

    #[test]
    fn check_wrong_syntax() {
        let res: Result<InitAttr, Error> = syn::parse_str("initialize => new");
        match res {
            Ok(_) => panic!("Expected to return error"),
            Err(_) => {}
        }
    }

    #[test]
    fn check_wrong_syntax2() {
        let res: Result<InitAttr, Error> = syn::parse_str("init -> new");
        match res {
            Ok(_) => panic!("Expected to return error"),
            Err(_) => {}
        }
    }

    #[test]
    fn simple_init() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let method: ImplItemMethod =
            syn::parse_str("pub fn method(k: &mut u64) -> Self { }").unwrap();

        let actual = process_init_method(&method, &impl_type, false).unwrap();
        let expected = quote!(
            #[cfg(target_arch = "wasm32")]
            #[no_mangle]
            pub extern "C" fn method() {
                near_bindgen::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
                let args: serde_json::Value = serde_json::from_slice(&near_bindgen::env::input().unwrap()).unwrap();
                let mut k: u64 = serde_json::from_value(args["k"].clone()).unwrap();
                let contract = Hello::method(&mut k, );
                near_bindgen::env::state_write(&contract);
            }
        );
        assert_eq!(expected.to_string(), actual.to_string());
    }
}
