use darling::{ast::NestedMeta, Error, FromMeta};
use proc_macro2::TokenStream;
use quote::quote;

#[derive(serde::Serialize, Default, FromMeta)]
pub struct ContractSourceMetadata {
    pub version: Option<String>,
    pub link: Option<String>,
    #[darling(multiple, rename = "standard")]
    pub standards: Vec<Standard>,
}

#[derive(FromMeta, serde::Serialize)]
pub struct Standard {
    pub standard: String,
    pub version: String,
}

#[derive(FromMeta)]
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
        let mut val = ContractSourceMetadata::default();
        populate_empty(&mut val);
        let metadata = serde_json::to_string(&val).expect("ContractSourceMetadata is parsable");

        return TokenStream::from(quote! {
            impl #struct_ident {
                pub const fn contract_source_metadata(&self) -> String {
                    #metadata
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

    let mut val = args.contract_metadata.unwrap();
    populate_empty(&mut val);
    let metadata = serde_json::to_string(&val).expect("ContractSourceMetadata is parsable");

    TokenStream::from(quote! {
        impl #struct_ident {
            pub const fn contract_source_metadata(&self) -> &'static str {
                #metadata
            }
        }
    })
}

fn populate_empty(metadata: &mut ContractSourceMetadata) {
    if metadata.version.is_none() {
        metadata.version = std::env::var("CARGO_PKG_VERSION").ok();
    }

    if metadata.link.is_none() {
        metadata.link = std::env::var("CARGO_PKG_REPOSITORY").ok();
    }

    if metadata.standards.is_empty() {
        metadata
            .standards
            .push(Standard { standard: "nep330".to_string(), version: "1.1.0".to_string() });
    }
}
