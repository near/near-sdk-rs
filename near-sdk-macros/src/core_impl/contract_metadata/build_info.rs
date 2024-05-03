#[derive(serde::Serialize)]
pub(crate) struct BuildInfo {
    build_environment: String,
    build_command: Vec<String>,
    contract_path: Option<String>,
    // pub source_code_snapshot: String,
    source_commit: String,
    source_git_url: String,
}

/// basic parsing errors
#[derive(Debug)]
pub(super) enum BuildInfoError {
    EmptyBuildEnvironment,
    EmptyBuildCommand,
    /// `None` value should be used instead for `contract_path`
    EmptyContractPath,
    EmptySourceSnapshot,
}

impl std::fmt::Display for BuildInfoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyBuildEnvironment => {
                write!(f, "`CARGO_NEAR_BUILD_ENVIRONMENT` is set, but it's set to empty string!")
            }
            Self::EmptyBuildCommand => {
                write!(f, "{}{}",
                    "`CARGO_NEAR_BUILD_COMMAND` is required, when `CARGO_NEAR_BUILD_ENVIRONMENT` is set,",
                    "but it's empty!"
                )
            }
            Self::EmptyContractPath => {
                write!(
                    f,
                    "{}{}",
                    "`CARGO_NEAR_CONTRACT_PATH` was provided,",
                    "but it's empty! It's required to be non-empty, when present!"
                )
            }
            Self::EmptySourceSnapshot => {
                write!(f, "{}{}",
                    "`CARGO_NEAR_SOURCE_CODE_GIT_URL` is required, when `CARGO_NEAR_BUILD_ENVIRONMENT` is set,",
                    "but it's empty!"
                )
            }
        }
    }
}

impl BuildInfo {
    pub(super) fn from_env() -> Result<Self, BuildInfoError> {
        let build_environment = std::env::var("CARGO_NEAR_BUILD_ENVIRONMENT")
            .map_err(|_| BuildInfoError::EmptyBuildEnvironment)?;
        if build_environment.is_empty() {
            return Err(BuildInfoError::EmptyBuildEnvironment);
        }
        let build_command = std::env::var("CARGO_NEAR_BUILD_COMMAND")
            .map_err(|_| BuildInfoError::EmptyBuildCommand)?;
        if build_command.is_empty() {
            return Err(BuildInfoError::EmptyBuildCommand);
        }
        let build_command =
            build_command.split_whitespace().map(|st| st.to_string()).collect::<Vec<_>>();
        if build_command.is_empty() {
            return Err(BuildInfoError::EmptyBuildCommand);
        }
        let contract_path = std::env::var("CARGO_NEAR_CONTRACT_PATH").ok();

        if contract_path.as_ref().is_some_and(|path| path.is_empty()) {
            return Err(BuildInfoError::EmptyContractPath);
        }
        let source_git_url = std::env::var("CARGO_NEAR_SOURCE_CODE_GIT_URL")
            .map_err(|_| BuildInfoError::EmptySourceSnapshot)?;
        if source_git_url.is_empty() {
            return Err(BuildInfoError::EmptySourceSnapshot);
        }
        let source_commit =
            std::env::var("CARGO_NEAR_SOURCE_CODE_COMMIT").unwrap_or("".to_string());

        Ok(Self { build_environment, build_command, contract_path, source_git_url, source_commit })
        // env_field!(self.source_commit, "CARGO_NEAR_SOURCE_CODE_COMMIT");
    }
}
