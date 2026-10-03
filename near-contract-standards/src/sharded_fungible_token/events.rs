use std::borrow::Cow;

use near_sdk::{near, serde::Deserialize, serde_with::DisplayFromStr, AccountIdRef};

#[cfg_attr(target_arch = "wasm32", must_use = "make sure to `.emit()` this event")]
#[near(event_json(standard = "nep636"))]
#[derive(Debug, Clone, Deserialize)]
pub enum SftEvent<'a> {
    #[event_version("1.0.0")]
    #[serde(rename = "sft_mint")]
    Mint(Cow<'a, [SftMint<'a>]>),

    #[event_version("1.0.0")]
    #[serde(rename = "sft_send")]
    Send(Cow<'a, [SftSend<'a>]>),

    #[event_version("1.0.0")]
    #[serde(rename = "sft_receive")]
    Receive(Cow<'a, [SftReceive<'a>]>),

    #[event_version("1.0.0")]
    #[serde(rename = "sft_burn")]
    Burn(Cow<'a, [SftBurn<'a>]>),
}

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct SftMint<'a> {
    pub owner_id: Cow<'a, AccountIdRef>,

    #[serde_as(as = "DisplayFromStr")]
    pub amount: u128,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<Cow<'a, str>>,
}

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct SftSend<'a> {
    pub receiver_id: Cow<'a, AccountIdRef>,

    #[serde_as(as = "DisplayFromStr")]
    pub amount: u128,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<Cow<'a, str>>,
}

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct SftReceive<'a> {
    pub sender_id: Cow<'a, AccountIdRef>,

    #[serde_as(as = "DisplayFromStr")]
    pub amount: u128,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<Cow<'a, str>>,
}

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct SftBurn<'a> {
    pub owner_id: Cow<'a, AccountIdRef>,

    #[serde_as(as = "DisplayFromStr")]
    pub amount: u128,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<Cow<'a, str>>,
}
