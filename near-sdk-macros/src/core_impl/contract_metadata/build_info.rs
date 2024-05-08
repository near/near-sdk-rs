#[derive(serde::Serialize)]
pub(crate) struct BuildInfo {
    build_environment: String,
    build_command: Vec<String>,
    contract_path: Option<String>,
    source_code_snapshot: String,
}

/// basic parsing errors
#[derive(Debug)]
pub(super) enum FieldEmptyError {
    BuildEnvironment,
    BuildCommand,
    /// `None` value should be used instead for `contract_path`
    ContractPath,
    SourceSnapshot,
}

#[allow(clippy::write_literal)]
impl std::fmt::Display for FieldEmptyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BuildEnvironment => {
                write!(f, "`CARGO_NEAR_BUILD_ENVIRONMENT` is set, but it's set to empty string!")
            }
            Self::BuildCommand => {
                write!(f, "{}{}",
                    "`CARGO_NEAR_BUILD_COMMAND` is required, when `CARGO_NEAR_BUILD_ENVIRONMENT` is set,",
                    "but it's empty!"
                )
            }
            Self::ContractPath => {
                write!(
                    f,
                    "{}{}",
                    "`CARGO_NEAR_CONTRACT_PATH` was provided,",
                    "but it's empty! It's required to be non-empty, when present!"
                )
            }
            Self::SourceSnapshot => {
                write!(f, "{}{}",
                    "`CARGO_NEAR_SOURCE_CODE_GIT_URL` is required, when `CARGO_NEAR_BUILD_ENVIRONMENT` is set,",
                    "but it's empty!"
                )
            }
        }
    }
}

impl BuildInfo {
    pub(super) fn from_env() -> Result<Self, FieldEmptyError> {
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
            FieldEmptyError::BuildEnvironment
        );
        let build_command = {
            env_field!(build_command, "CARGO_NEAR_BUILD_COMMAND", FieldEmptyError::BuildCommand);
            let build_command =
                build_command.split_whitespace().map(|st| st.to_string()).collect::<Vec<_>>();
            if build_command.is_empty() {
                return Err(FieldEmptyError::BuildCommand);
            }
            build_command
        };
        env_field!(
            source_code_snapshot,
            "CARGO_NEAR_SOURCE_CODE_SNAPSHOT",
            FieldEmptyError::SourceSnapshot
        );
        let contract_path = std::env::var("CARGO_NEAR_CONTRACT_PATH").ok();

        if contract_path.as_ref().is_some_and(|path| path.is_empty()) {
            return Err(FieldEmptyError::ContractPath);
        }

        Ok(Self { build_environment, build_command, contract_path, source_code_snapshot })
    }
}
