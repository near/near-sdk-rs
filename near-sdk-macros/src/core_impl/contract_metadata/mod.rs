use darling::{ast::NestedMeta, Error, FromMeta};
use proc_macro2::TokenStream;
use quote::quote;

#[derive(FromMeta)]
struct MacroConfig {
    contract_metadata: Option<ContractMetadata>,
}

#[derive(serde::Serialize, Default, FromMeta)]
struct ContractMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    link: Option<String>,
    #[darling(multiple, rename = "standard")]
    standards: Vec<Standard>,
}

#[derive(FromMeta, serde::Serialize)]
struct Standard {
    standard: String,
    version: String,
}

impl ContractMetadata {
    fn populate(mut self) -> Self {
        if self.version.is_none() {
            self.version = std::env::var("CARGO_PKG_VERSION").ok();
        }

        if self.link.is_none() {
            self.link = std::env::var("CARGO_PKG_REPOSITORY").ok();
        }

        // adding nep330 if it is not present
        if self.standards.is_empty()
            || self
                .standards
                .iter()
                .find(|s| s.standard.to_ascii_lowercase().eq("nep330"))
                .clone()
                .is_none()
        {
            self.standards
                .push(Standard { standard: "nep330".to_string(), version: "1.1.0".to_string() });
        }

        self
    }
}

/// Allows for the injection of the contract source metadata infomation into the contract code as
/// a constant.
pub(crate) fn contract_source_metadata_const(attr: proc_macro::TokenStream) -> TokenStream {
    if attr.to_string().is_empty() {
        let metadata = serde_json::to_string(&ContractMetadata::default().populate())
            .expect("ContractMetadata implements Serialize");

        return quote! {
           pub const CONTRACT_SOURCE_METADATA: &'static str = #metadata;
        };
    }

    let attr_args = match NestedMeta::parse_meta_list(attr.into()) {
        Ok(v) => v,
        Err(e) => {
            return Error::from(e).write_errors();
        }
    };

    let args = match MacroConfig::from_list(&attr_args) {
        Ok(v) => v,
        Err(e) => {
            return e.write_errors();
        }
    };

    let metadata = serde_json::to_string(
        &args
            .contract_metadata
            .expect("Attribute input must be present given standard was followed")
            .populate(),
    )
    .expect("ContractMetadata implements Serialize");

    quote! {
        const CONTRACT_SOURCE_METADATA: &'static str = #metadata;
    }
}
