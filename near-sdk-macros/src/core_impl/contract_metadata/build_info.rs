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
            .ok()
            .filter(|build_environment| !build_environment.is_empty())
            .ok_or(ERR_EMPTY_BUILD_ENVIRONMENT.to_string())?;

        let build_command = std::env::var("CARGO_NEAR_BUILD_COMMAND")
            .ok()
            .filter(|build_command| !build_command.is_empty())
            .map(|build_command| {
                build_command.split_whitespace().map(|st| st.to_string()).collect::<Vec<_>>()
            })
            .filter(|build_command| !build_command.is_empty())
            .ok_or(ERR_EMPTY_BUILD_COMMAND.to_string())?;
        let source_code_snapshot = std::env::var("CARGO_NEAR_SOURCE_CODE_SNAPSHOT")
            .ok()
            .filter(|source_code_snapshot| !source_code_snapshot.is_empty())
            .ok_or(ERR_UNSET_OR_EMPTY_SOURCE_SNAPSHOT.to_string())?;
        let contract_path = std::env::var("CARGO_NEAR_CONTRACT_PATH")
            .map_err(|_| ERR_UNSET_CONTRACT_PATH.to_string())?;

        Ok(Self { build_environment, build_command, contract_path, source_code_snapshot })
    }
}
