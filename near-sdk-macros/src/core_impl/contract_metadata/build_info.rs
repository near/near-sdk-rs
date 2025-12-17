use std::env::VarError;

#[derive(serde::Serialize)]
pub(crate) struct BuildInfo {
    build_environment: String,
    build_command: Vec<String>,
    contract_path: String,
    source_code_snapshot: String,
    output_wasm_path: Option<String>,
}
pub mod nep330_keys {
    pub const BUILD_ENVIRONMENT: &str = "NEP330_BUILD_INFO_BUILD_ENVIRONMENT";
    pub const BUILD_COMMAND: &str = "NEP330_BUILD_INFO_BUILD_COMMAND";
    pub const SOURCE_CODE_SNAPSHOT: &str = "NEP330_BUILD_INFO_SOURCE_CODE_SNAPSHOT";
    pub const CONTRACT_PATH: &str = "NEP330_BUILD_INFO_CONTRACT_PATH";
    pub const OUTPUT_WASM_PATH: &str = "NEP330_BUILD_INFO_OUTPUT_WASM_PATH";
}

const ERR_EMPTY_BUILD_ENVIRONMENT: &str = "`NEP330_BUILD_INFO_BUILD_ENVIRONMENT` is set, \
                                            but it's set to empty string!";
const ERR_EMPTY_BUILD_COMMAND: &str = "`NEP330_BUILD_INFO_BUILD_COMMAND` is required, \
                                        when `NEP330_BUILD_INFO_BUILD_ENVIRONMENT` is set, \
                                        but it's either not set or empty!";
const ERR_PARSE_BUILD_COMMAND: &str = "problem parsing `NEP330_BUILD_INFO_BUILD_COMMAND` value";

const ERR_UNSET_CONTRACT_PATH: &str = "`NEP330_BUILD_INFO_CONTRACT_PATH` was provided, \
                                        but it's not set!";

const ERR_UNSET_OR_EMPTY_SOURCE_SNAPSHOT: &str = "`NEP330_BUILD_INFO_SOURCE_CODE_SNAPSHOT` is \
                                                    required, when `NEP330_BUILD_INFO_BUILD_ENVIRONMENT` \
                                                    is set, but it's either not set or empty!";

const ERR_INVALID_UNICODE_OUTPUT_WASM_PATH: &str = "`NEP330_BUILD_INFO_OUTPUT_WASM_PATH` was \
                                                    provided, but it contained invalid UTF8!";

impl BuildInfo {
    pub(super) fn from_env() -> Result<Self, String> {
        let build_environment = std::env::var(nep330_keys::BUILD_ENVIRONMENT)
            .ok()
            .filter(|build_environment| !build_environment.is_empty())
            .ok_or(ERR_EMPTY_BUILD_ENVIRONMENT.to_string())?;

        let build_command = std::env::var(nep330_keys::BUILD_COMMAND)
            .ok()
            .filter(|build_command| !build_command.is_empty())
            .ok_or(ERR_EMPTY_BUILD_COMMAND.to_string())?;
        let build_command: Vec<String> = serde_json::from_str(&build_command)
            .map_err(|err| format!("{ERR_PARSE_BUILD_COMMAND}: {err}"))?;

        let source_code_snapshot = std::env::var(nep330_keys::SOURCE_CODE_SNAPSHOT)
            .ok()
            .filter(|source_code_snapshot| !source_code_snapshot.is_empty())
            .ok_or(ERR_UNSET_OR_EMPTY_SOURCE_SNAPSHOT.to_string())?;
        let contract_path = std::env::var(nep330_keys::CONTRACT_PATH)
            .map_err(|_| ERR_UNSET_CONTRACT_PATH.to_string())?;

        let output_wasm_path = match std::env::var(nep330_keys::OUTPUT_WASM_PATH) {
            Ok(path) => Some(path),
            Err(VarError::NotPresent) => None,
            Err(VarError::NotUnicode(_err)) => {
                return Err(ERR_INVALID_UNICODE_OUTPUT_WASM_PATH.to_string());
            }
        };
        Ok(Self {
            build_environment,
            build_command,
            contract_path,
            source_code_snapshot,
            output_wasm_path,
        })
    }
}
