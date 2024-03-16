use darling::{ast::NestedMeta, Error, FromMeta};
use proc_macro2::TokenStream;
use quote::quote;

#[derive(FromMeta)]
struct MacroConfig {
    contract_metadata: Option<ContractMetadata>,
}

#[derive(serde::Serialize, Default, FromMeta)]
pub(crate) struct ContractMetadata {
    version: Option<String>,
    link: Option<String>,
    #[darling(multiple, rename = "standard")]
    standards: Vec<Standard>,
}

impl quote::ToTokens for ContractMetadata {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let version = &self.version;
        let link = &self.link;
        let mut standards = quote! {};
        let standards_vec = &self.standards;
        for standard in standards_vec {
            let standard_name = &standard.standard;
            let standard_version = &standard.version;
            standards = quote! {
                #standards
                standard(standard = #standard_name, version = #standard_version),
            };
        }
        tokens.extend(quote! {
            contract_metadata(
                version = #version,
                link = #link,
                #standards
            )
        })
    }
}

#[derive(FromMeta, serde::Serialize)]
struct Standard {
    standard: String,
    version: String,
}

impl ContractMetadata {
    fn populate(mut self) -> Self {
        if self.version.is_none() {
            let version = std::env::var("CARGO_PKG_VERSION").unwrap_or(String::from(""));
            if !version.is_empty() {
                self.version = Some(version);
            }
        }

        if self.link.is_none() {
            let repo = std::env::var("CARGO_PKG_REPOSITORY").unwrap_or(String::from(""));
            if !repo.is_empty() {
                self.link = Some(repo);
            }
        }

        // adding nep330 if it is not present
        if self.standards.is_empty()
            || self.standards.iter().all(|s| s.standard.to_ascii_lowercase() != "nep330")
        {
            self.standards
                .push(Standard { standard: "nep330".to_string(), version: "1.1.0".to_string() });
        }

        self
    }
}

/// Allows for the injection of the contract source metadata information into the contract code as
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
