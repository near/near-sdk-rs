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
        macro_rules! env_field {
            ($field: ident, $env_key: expr, $error: expr) => {
                let $field = std::env::var($env_key).map_err(|_| $error)?;
                if $field.is_empty() {
                    return Err($error);
                }
            };
        }

        env_field!(
            build_environment,
            "CARGO_NEAR_BUILD_ENVIRONMENT",
            FieldError::EmptyBuildEnvironment
        );
        let build_command = {
            env_field!(
                build_command,
                "CARGO_NEAR_BUILD_COMMAND",
                FieldError::UnsetOrEmptyBuildCommand
            );
            let build_command =
                build_command.split_whitespace().map(|st| st.to_string()).collect::<Vec<_>>();
            if build_command.is_empty() {
                return Err(FieldError::UnsetOrEmptyBuildCommand);
            }
            build_command
        };
        env_field!(
            source_code_snapshot,
            "CARGO_NEAR_SOURCE_CODE_SNAPSHOT",
            FieldError::UnsetOrEmptySourceSnapshot
        );
        let contract_path =
            std::env::var("CARGO_NEAR_CONTRACT_PATH").map_err(|_| FieldError::UnsetContractPath)?;

        Ok(Self { build_environment, build_command, contract_path, source_code_snapshot })
    }
}
