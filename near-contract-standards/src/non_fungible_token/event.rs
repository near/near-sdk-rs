// use std::fmt;

use serde::Serialize;
use serde_with::skip_serializing_none;

#[allow(non_camel_case_types)]
#[derive(Serialize)]
enum Event {
    nft_mint,
    nft_burn,
    nft_transfer,
}

// impl fmt::Display for Event {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         use Event::*;
//         f.write_str(match self {
//             NftMint => "nft_mint",
//             NftBurn => "nft_burn",
//             NftTransfer => "nft_transfer",
//         })
//     }
// }
// impl Serialize for Event {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: serde::Serializer,
//     {
//         serializer.serialize_str(&self.to_string())
//     }
// }

#[skip_serializing_none]
#[derive(Serialize)]
struct NftMintLog {
    owner_id: String,
    token_ids: Vec<String>,
    memo: Option<String>,
}

#[derive(Serialize)]
struct NftBurnLog {
    owner_id: String,
    authorized_id: Option<String>,
    token_ids: Vec<String>,
    memo: Option<String>,
}

#[derive(Serialize)]
struct NftTransferLog {
    authorized_id: Option<String>,
    old_owner_id: String,
    new_owner_id: String,
    token_ids: Vec<String>,
    memo: Option<String>,
}

enum EventLogData {
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
            Mint(log)  => log.serialize(serializer),
            Burn(log)  => log.serialize(serializer),
            Transfer(log) => log.serialize(serializer),
        }
    }
}

impl From<NftMintLog> for EventLogData {
    fn from(log: NftMintLog) -> Self {
        EventLogData::Mint(log)
    }
}

#[derive(Serialize)]
struct EventLog {
    standard: String,
    version: String,
    event: Event,
    data: Vec<EventLogData>,
}

impl EventLog {
    pub fn new(standard: String, version: String, event: Event, data: Vec<EventLogData>) -> Self {
        Self { standard, version, event, data }
    }

    pub fn nft_mint(data: Vec<NftMintLog>) -> Self {
        EventLog::new(
            "nep171".to_string(),
            "1.0.0".to_string(),
            Event::nft_mint,
            data.into_iter().map(|log| log.into()).collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_log() {
        let token_ids = vec!["0", "1"].iter().map(|t| t.to_string()).collect();
        let mint_log = NftMintLog { owner_id: "bob".to_string(), token_ids, memo: None };
        // let token_strs: Vec<String> = token_ids.iter().map(|id| format!("\"{}\"", id)).collect();
        println!("{}", serde_json::to_string(&mint_log).unwrap());
        let event_log = EventLog::nft_mint(vec![mint_log]);
        println!("{}", serde_json::to_string(&event_log).unwrap());
        // format!(
        //     r#"{{
        // "standard": "nep171",
        // "version": "1.0.0",
        // "event": "nft_mint",
        // "data": [
        //     {{"owner_id": "{}", "token_ids": [{}]}}
        // ]
        //     }}"#,
        //     owner_id,
        //     token_strs.join(",")
        // )
    }
}

// pub fn nft_mint(owner_id: &str, token_ids: Vec<String>) -> String {
