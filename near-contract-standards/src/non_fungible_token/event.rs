// use std::fmt;

use serde::Serialize;
use serde_with::skip_serializing_none;

#[allow(non_camel_case_types)]
#[derive(Serialize)]
pub enum Event {
    nft_mint,
    nft_burn,
    nft_transfer,
}

#[skip_serializing_none]
#[derive(Serialize)]
pub struct NftMintLog {
    owner_id: String,
    token_ids: Vec<String>,
    memo: Option<String>,
}

#[skip_serializing_none]
#[derive(Serialize)]
pub struct NftBurnLog {
    owner_id: String,
    authorized_id: Option<String>,
    token_ids: Vec<String>,
    memo: Option<String>,
}

#[skip_serializing_none]
#[derive(Serialize)]
pub struct NftTransferLog {
    authorized_id: Option<String>,
    old_owner_id: String,
    new_owner_id: String,
    token_ids: Vec<String>,
    memo: Option<String>,
}

pub enum EventLogData {
    Mint(NftMintLog),
    Burn(NftBurnLog),
    Transfer(NftTransferLog),
}

impl Serialize for EventLogData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use EventLogData::*;
        match self {
            Mint(log) => log.serialize(serializer),
            Burn(log) => log.serialize(serializer),
            Transfer(log) => log.serialize(serializer),
        }
    }
}

impl From<NftMintLog> for EventLogData {
    fn from(log: NftMintLog) -> Self {
        EventLogData::Mint(log)
    }
}

impl From<NftTransferLog> for EventLogData {
    fn from(log: NftTransferLog) -> Self {
        EventLogData::Transfer(log)
    }
}

impl From<NftBurnLog> for EventLogData {
    fn from(log: NftBurnLog) -> Self {
        EventLogData::Burn(log)
    }
}

#[derive(Serialize)]
pub struct EventLog {
    standard: String,
    version: String,
    event: Event,
    data: Vec<EventLogData>,
}

impl std::fmt::Display for EventLog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&serde_json::to_string(self).unwrap())
    }
}

impl EventLog {
    pub fn new(standard: String, version: String, event: Event, data: Vec<EventLogData>) -> Self {
        Self { standard, version, event, data }
    }

    pub fn new_nep171(event: Event, data: Vec<EventLogData>) -> Self {
        EventLog::new("nep171".to_string(), "1.0.0".to_string(), event, data)
    }

    pub fn nft_burn(data: Vec<NftBurnLog>) -> Self {
        EventLog::new_nep171(Event::nft_burn, data.into_iter().map(|log| log.into()).collect())
    }

    pub fn nft_mint(data: Vec<NftMintLog>) -> Self {
        EventLog::new_nep171(Event::nft_mint, data.into_iter().map(|log| log.into()).collect())
    }

    pub fn nft_transfer(data: Vec<NftTransferLog>) -> Self {
        EventLog::new_nep171(Event::nft_transfer, data.into_iter().map(|log| log.into()).collect())
    }

    pub fn log(&self) {
        near_sdk::env::log_str(&format!("EVENT_JSON:{}", self));
    }

    pub fn log_nft_mint(owner_id: String, token_ids: Vec<String>, memo: Option<String>) {
        EventLog::nft_mint(vec![NftMintLog { owner_id, token_ids, memo }]).log();
    }
    pub fn log_nft_transfer(
        old_owner_id: String,
        new_owner_id: String,
        token_ids: Vec<String>,
        memo: Option<String>,
        authorized_id: Option<String>,
    ) {
        EventLog::nft_transfer(vec![NftTransferLog {
            authorized_id,
            old_owner_id,
            new_owner_id,
            token_ids,
            memo,
        }])
        .log();
    }

    pub fn log_nft_burn(
        owner_id: String,
        authorized_id: Option<String>,
        token_ids: Vec<String>,
        memo: Option<String>,
    ) {
        EventLog::nft_burn(vec![NftBurnLog { owner_id, authorized_id, token_ids, memo }]).log();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nft_mint() {
        let owner_id = "bob".to_string();
        let token_ids = vec!["0", "1"].iter().map(|t| t.to_string()).collect();
        let mint_log = NftMintLog { owner_id, token_ids, memo: None };
        let event_log = EventLog::nft_mint(vec![mint_log]);
        assert_eq!(
            serde_json::to_string(&event_log).unwrap(),
            r#"{"standard":"nep171","version":"1.0.0","event":"nft_mint","data":[{"owner_id":"bob","token_ids":["0","1"]}]}"#
        );
    }

    #[test]
    fn nft_burn() {
      let owner_id = "bob".to_string();
      let token_ids = vec!["0", "1"].iter().map(|t| t.to_string()).collect();
      let log = EventLog::nft_burn(vec![NftBurnLog { owner_id, authorized_id: None, token_ids, memo: None }]).to_string();
      assert_eq!(log, r#"{"standard":"nep171","version":"1.0.0","event":"nft_burn","data":[{"owner_id":"bob","token_ids":["0","1"]}]}"#);
    }

    #[test]
    fn nft_transfer() {
      let old_owner_id = "bob".to_string();
      let new_owner_id = "alice".to_string();
      let token_ids = vec!["0", "1"].iter().map(|t| t.to_string()).collect();
      let log = EventLog::nft_transfer(vec![NftTransferLog { old_owner_id, new_owner_id, authorized_id: None, token_ids, memo: None }]).to_string();
      assert_eq!(log, r#"{"standard":"nep171","version":"1.0.0","event":"nft_transfer","data":[{"old_owner_id":"bob","new_owner_id":"alice","token_ids":["0","1"]}]}"#);
    }
}


