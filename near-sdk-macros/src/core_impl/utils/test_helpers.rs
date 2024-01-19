use prettyplease;
use proc_macro2::TokenStream;
use quote::quote;

pub fn pretty_print_syn_str(input: &TokenStream) -> syn::Result<String> {
    let input = format!("{}", quote!(#input));
    let syn_file = syn::parse_str::<syn::File>(&input)?;

    Ok(prettyplease::unparse(&syn_file))
}

macro_rules! local_insta_assert_snapshot {
    ($value:expr) => {{

        insta::with_settings!({prepend_module_to_snapshot => false}, {
            insta::assert_snapshot!($value);
        });
    }};
}
pub(crate) use local_insta_assert_snapshot;
