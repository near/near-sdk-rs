use near_sdk::serde::{Deserialize, Serialize};

//enum that represents the data type of the EventLogJson. 
//the enum can either be an NftMintLog or an NftTransferLog
#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub enum EventLogJsonData {
    NftMintLog(Vec<NftMintLog>),
    NftTransferLog(Vec<NftTransferLog>),
}

// Interface to capture data 
// about an event
// Arguments
// * `standard`: name of standard e.g. nep171
// * `version`: e.g. 1.0.0
// * `event`: string
// * `data`: associate event data
#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct EventLogJson {
    pub standard: String, 
    pub version: String, 
    pub event: String,
    pub data: EventLogJsonData
}

// An event log to capture token minting
// Arguments
// * `owner_id`: "account.near"
// * `token_ids`: ["1", "abc"]
// * `memo`: optional message
#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct NftMintLog {
    pub owner_id: String, 
    pub token_ids: Vec<String>, 

    //Only serialize if the option is not none
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>
}

// An event log to capture token transfer
// Arguments
// * `authorized_id`: approved account to transfer
// * `old_owner_id`: "owner.near"
// * `new_owner_id`: "receiver.near"
// * `token_ids`: ["1", "12345abc"]
// * `memo`: optional message
#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct NftTransferLog {
    //Only serialize if the option is not none
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorized_id: Option<String>, 
    pub old_owner_id: String, 
    pub new_owner_id: String, 
    pub token_ids: Vec<String>, 
    //Only serialize if the option is not none
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>
}