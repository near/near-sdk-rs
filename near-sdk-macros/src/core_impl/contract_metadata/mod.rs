#![allow(clippy::manual_unwrap_or_default)]

use darling::FromMeta;
use proc_macro2::TokenStream;
use quote::quote;

mod nep330_keys {

    pub const LINK: &str = "NEP330_LINK";
    pub const VERSION: &str = "NEP330_VERSION";
}
mod build_info;

const CARGO_PKG_REPOSITORY_KEY: &str = "CARGO_PKG_REPOSITORY";
const CARGO_PKG_VERSION_KEY: &str = "CARGO_PKG_VERSION";

const STANDARD_NEP330: &str = "nep330";
const STANDARD_NEP330_VERSION: &str = "1.3.0";

#[derive(serde::Serialize, Default, FromMeta)]
pub(crate) struct ContractMetadata {
    version: Option<String>,
    link: Option<String>,

    #[darling(multiple, rename = "standard")]
    standards: Vec<Standard>,

    #[darling(skip)]
    build_info: Option<build_info::BuildInfo>,
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

        // This is necessary, because using a simple `version = #version`, will lead to macros tokenizing `version = None` to `version = `, which
        // cannot be parsed as meta and is considered invalid.
        let version_tokens = version.as_ref().map(|v| quote! { version = #v, });
        let link_tokens = link.as_ref().map(|l| quote! { link = #l, });

        tokens.extend(quote! {
            #version_tokens
            #link_tokens
            #standards
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
        if self.link.is_none() {
            let field_val = std::env::var(nep330_keys::LINK)
                .or(std::env::var(CARGO_PKG_REPOSITORY_KEY))
                .unwrap_or(String::from(""));
            if !field_val.is_empty() {
                self.link = Some(field_val);
            }
        }
        if self.version.is_none() {
            let field_val = std::env::var(nep330_keys::VERSION)
                .or(std::env::var(CARGO_PKG_VERSION_KEY))
                .unwrap_or(String::from(""));
            if !field_val.is_empty() {
                self.version = Some(field_val);
            }
        }

        // adding nep330 if it is not present
        if self.standards.is_empty()
            || self.standards.iter().all(|s| !s.standard.eq_ignore_ascii_case(STANDARD_NEP330))
        {
            self.standards.push(Standard {
                standard: STANDARD_NEP330.to_string(),
                version: STANDARD_NEP330_VERSION.to_string(),
            });
        }

        if std::env::var(build_info::nep330_keys::BUILD_ENVIRONMENT).is_ok() {
            self.build_info = Some(
                build_info::BuildInfo::from_env()
                    .expect("Build Details Extension field not provided or malformed"),
            );
        }

        self
    }

    /// Allows for the injection of the contract source metadata
    /// information into the contract code as a constant.
    pub fn contract_source_metadata_const(self) -> TokenStream {
        let metadata =
            serde_json::to_string(&self.populate()).expect("ContractMetadata implements Serialize");

        quote! {
            const CONTRACT_SOURCE_METADATA: &'static str = #metadata;
        }
    }
}
