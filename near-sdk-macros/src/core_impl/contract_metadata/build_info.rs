#[derive(serde::Serialize)]
pub(crate) struct BuildInfo {
    build_environment: String,
    build_command: Vec<String>,
    contract_path: String,
    source_code_snapshot: String,
}

const ERR_EMPTY_BUILD_ENVIRONMENT: &str = "`CARGO_NEAR_BUILD_ENVIRONMENT` is set, \
                                            but it's set to empty string!";
const ERR_EMPTY_BUILD_COMMAND: &str = "`CARGO_NEAR_BUILD_COMMAND` is required, \
                                        when `CARGO_NEAR_BUILD_ENVIRONMENT` is set, \
                                        but it's either not set or empty!";

const ERR_UNSET_CONTRACT_PATH: &str = "`CARGO_NEAR_CONTRACT_PATH` was provided, \
                                        but it's not set!";

const ERR_UNSET_OR_EMPTY_SOURCE_SNAPSHOT: &str = "`CARGO_NEAR_SOURCE_CODE_GIT_URL` is \
                                                    required, when `CARGO_NEAR_BUILD_ENVIRONMENT` \
                                                    is set, but it's either not set or empty!";

impl BuildInfo {
    pub(super) fn from_env() -> Result<Self, String> {
        let build_environment = std::env::var("CARGO_NEAR_BUILD_ENVIRONMENT")
            .map_err(|_| ERR_EMPTY_BUILD_ENVIRONMENT.to_string())?;
        if build_environment.is_empty() {
            return Err(ERR_EMPTY_BUILD_ENVIRONMENT.to_string());
        }

        let build_command = {
            let build_command = std::env::var("CARGO_NEAR_BUILD_COMMAND")
                .map_err(|_| ERR_EMPTY_BUILD_COMMAND.to_string())?;
            if build_command.is_empty() {
                return Err(ERR_EMPTY_BUILD_COMMAND.to_string());
            }
            let build_command =
                build_command.split_whitespace().map(|st| st.to_string()).collect::<Vec<_>>();
            if build_command.is_empty() {
                return Err(ERR_EMPTY_BUILD_COMMAND.to_string());
            }
            build_command
        };
        let source_code_snapshot = std::env::var("CARGO_NEAR_SOURCE_CODE_SNAPSHOT")
            .map_err(|_| ERR_UNSET_OR_EMPTY_SOURCE_SNAPSHOT.to_string())?;
        if source_code_snapshot.is_empty() {
            return Err(ERR_UNSET_OR_EMPTY_SOURCE_SNAPSHOT.to_string());
        }
        let contract_path = std::env::var("CARGO_NEAR_CONTRACT_PATH")
            .map_err(|_| ERR_UNSET_CONTRACT_PATH.to_string())?;

        Ok(Self { build_environment, build_command, contract_path, source_code_snapshot })
    }
}
