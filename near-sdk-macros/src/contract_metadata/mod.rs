use darling::{ast::NestedMeta, Error, FromMeta};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
//use syn::parse_quote;

#[derive(Clone, Debug)]
pub struct ContractSourceMetadata {
    pub version: Option<String>,
    pub link: Option<String>,
    pub standards: Vec<Standard>,
}

impl FromMeta for ContractSourceMetadata {
    fn from_list(v: &[NestedMeta]) -> Result<Self, Error> {
        let mut version = None;
        let mut link = None;
        let mut standards = vec![];

        // TODO: parse the NestedMeta list and populate the version, link, and standards fields.

        Ok(Self { version, link, standards })
    }
}

#[derive(Clone, Debug, FromMeta)]
pub struct Standard {
    pub standard: String,
    pub version: String,
}

impl ToTokens for Standard {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let standard = &self.standard;
        let version = &self.version;
        tokens.extend(quote! {
            ::near_contract_standards::Standard {
                standard: #standard,
                version: #version,
            }
        });
    }
}

#[derive(Default, FromMeta, Clone, Debug)]
struct MetadataConfig {
    pub contract_metadata: Option<ContractSourceMetadata>,
}

/// This function is used to extract/populate the contract metadata to the contract.
pub(crate) fn contract_metadata(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> TokenStream {
    let struct_ident = match syn::parse::<syn::ItemStruct>(item.into()) {
        Ok(v) => v.ident,
        Err(e) => {
            return TokenStream::from(e.to_compile_error());
        }
    };

    // short circuit to the default implementation if the contract_metadata attribute is not present
    if !attr.to_string().contains("contract_metadata") {
        return TokenStream::from(quote! {
            impl #struct_ident {
                pub fn contract_source_metadata(&self) -> ::near_contract_standards::ContractSourceMetadata {
                    ::near_contract_standards::ContractSourceMetadata::default()
                }
            }
        });
    }

    let attr_args = match NestedMeta::parse_meta_list(attr.into()) {
        Ok(v) => v,
        Err(e) => {
            return proc_macro2::TokenStream::from(Error::from(e).write_errors());
        }
    };

    let args = match MetadataConfig::from_list(&attr_args) {
        Ok(v) => v,
        Err(e) => {
            return TokenStream::from(e.write_errors());
        }
    };

    let ver = args.contract_metadata.clone().map(|v| v.version).flatten();
    let link = args.contract_metadata.clone().map(|v| v.link).flatten();
    let standards = args.contract_metadata.map(|v| v.standards).unwrap_or(vec![]);

    // populate the impl with the parsed data.
    TokenStream::from(quote! {
        impl #struct_ident {
            pub fn contract_source_metadata(&self) -> ::near_contract_standards::ContractSourceMetadata {
                ::near_contract_standards::ContractSourceMetadata {
                    version: #ver,
                    link: #link,
                    standards: #(#standards)*
                }
            }
        }
    })
}
