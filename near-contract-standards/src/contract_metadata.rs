/// The contract source metadata is a standard interface that allows auditing and viewing source code for a deployed smart contract.
#[derive(serde::Deserialize, Debug)]
pub struct ContractSourceMetadata {
    pub version: Option<String>,
    pub link: Option<String>,
    pub standards: Vec<Standard>,
}

#[derive(serde:: Deserialize, Debug)]
pub struct Standard {
    pub standard: String,
    pub version: String,
}

impl ContractSourceMetadata {
    /// Parses the contract source metadata from the returned JSON string.
    /// Assuming you have a contract at hand, the data is expected to be parsed like so:
    /// ```ignore, no_run
    /// let worker = near_workspaces::sandbox().await?;
    /// let contract = worker.dev_deploy(CONTRACT_CODE).await?;
    ///
    /// let payload = contract
    ///     .view("contract_source_metadata")
    ///     .await?
    ///     .json::<String>()?;
    ///
    /// let metadata = ContractSourceMetadata::from_payload(&payload)?;
    /// ```
    pub fn from_payload(payload: &str) -> anyhow::Result<Self> {
        serde_json::from_str(payload)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize contract source metadata: {}", e))
    }
}
