#[allow(unused_imports)]
use syn::export::ToTokens;
use syn::export::TokenStream2;
use syn::{ItemStruct, Fields, FieldsNamed};
use quote::quote;

pub fn generate_struct(input: ItemStruct) -> TokenStream2 {
    let ItemStruct { ident, attrs, .. } = input;
    let non_bindgen_attrs = attrs.iter().fold(TokenStream2::new(), |acc, value| {
        quote! {
                #acc
                #value
            }
    });
    let fields = match &input.fields {
          Fields:: Named(FieldsNamed {
              named,
              ..
                         }) => {
              named.to_token_stream()
          },
           _ => quote!{}
    };

    quote! {
            #non_bindgen_attrs
            pub struct #ident {
                #fields
                pub contract_id: String,
            }
        }

}

#[rustfmt::skip]
#[cfg(test)]
mod tests {
    use syn::ItemStruct;
    use quote::quote;

    use crate::generate_struct;


    #[test]
    fn simple_struct() {
        if let Ok(input) = syn::parse_str::<ItemStruct>(
"\
#[derive(Copy)]\
struct Hello {\
    a_field: String,\
    another_field: u32,\
}"
        ) {
            println!("{:?}", input.clone());
            // println!("{:?}", generate_struct(input).to_string())

            //     let mut method: ImplItemMethod = syn::parse_str("fn method(&self) { }").unwrap();
            // let method_info = ImplItemMethodInfo::new(&mut method, impl_type).unwrap();
            let actual = generate_struct(input);
            let expected = quote!(
                #[derive(Copy)]
                struct Hello {
                   a_field: String,
                   another_field: u32,
                   contract_id: String,
                }
            );
            assert_eq!(expected.to_string(), actual.to_string());
        } else {
            panic!("oops couldn't parse a struct")
        }
    }
}
