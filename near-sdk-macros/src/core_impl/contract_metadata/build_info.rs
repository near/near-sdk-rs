#[derive(serde::Serialize)]
pub(crate) struct BuildInfo {
    build_environment: String,
    build_command: Vec<String>,
    contract_path: String,
    source_code_snapshot: String,
}

/// basic parsing errors
#[derive(Debug)]
pub(super) enum FieldError {
    EmptyBuildEnvironment,
    UnsetOrEmptyBuildCommand,
    /// `None` value should be used instead for `contract_path`
    UnsetContractPath,
    UnsetOrEmptySourceSnapshot,
}

#[allow(clippy::write_literal)]
impl std::fmt::Display for FieldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyBuildEnvironment => {
                write!(f, "`CARGO_NEAR_BUILD_ENVIRONMENT` is set, but it's set to empty string!")
            }
            Self::UnsetOrEmptyBuildCommand => {
                write!(f, "{}{}",
                    "`CARGO_NEAR_BUILD_COMMAND` is required, when `CARGO_NEAR_BUILD_ENVIRONMENT` is set, ",
                    "but it's either not set or empty!"
                )
            }
            Self::UnsetContractPath => {
                write!(f, "{}{}", "`CARGO_NEAR_CONTRACT_PATH` was provided, ", "but it's not set!")
            }
            Self::UnsetOrEmptySourceSnapshot => {
                write!(f, "{}{}",
                    "`CARGO_NEAR_SOURCE_CODE_GIT_URL` is required, when `CARGO_NEAR_BUILD_ENVIRONMENT` is set, ",
                    "but it's either not set or empty!"
                )
            }
        }
    }
}

impl BuildInfo {
    pub(super) fn from_env() -> Result<Self, FieldError> {
        let build_environment = std::env::var("CARGO_NEAR_BUILD_ENVIRONMENT")
            .map_err(|_| FieldError::EmptyBuildEnvironment)?;
        if build_environment.is_empty() {
            return Err(FieldError::EmptyBuildEnvironment);
        }

        let build_command = {
            let build_command = std::env::var("CARGO_NEAR_BUILD_COMMAND")
                .map_err(|_| FieldError::UnsetOrEmptyBuildCommand)?;
            if build_command.is_empty() {
                return Err(FieldError::UnsetOrEmptyBuildCommand);
            }
            let build_command =
                build_command.split_whitespace().map(|st| st.to_string()).collect::<Vec<_>>();
            if build_command.is_empty() {
                return Err(FieldError::UnsetOrEmptyBuildCommand);
            }
            build_command
        };
        let source_code_snapshot = std::env::var("CARGO_NEAR_SOURCE_CODE_SNAPSHOT")
            .map_err(|_| FieldError::UnsetOrEmptySourceSnapshot)?;
        if source_code_snapshot.is_empty() {
            return Err(FieldError::UnsetOrEmptySourceSnapshot);
        }
        let contract_path =
            std::env::var("CARGO_NEAR_CONTRACT_PATH").map_err(|_| FieldError::UnsetContractPath)?;

        Ok(Self { build_environment, build_command, contract_path, source_code_snapshot })
    }
}
