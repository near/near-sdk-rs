use std::borrow::Cow;

use near_sdk::{near, serde::Deserialize, serde_with::DisplayFromStr, AccountIdRef};

#[cfg_attr(target_arch = "wasm32", must_use = "make sure to `.emit()` this event")]
#[near(event_json(standard = "nep636"))]
#[derive(Debug, Clone, Deserialize)]
pub enum SftEvent<'a> {
    #[event_version("1.0.0")]
    #[serde(rename = "sft_minted")]
    Minted(Cow<'a, [SftMinted<'a>]>),

    #[event_version("1.0.0")]
    #[serde(rename = "sft_sent")]
    Sent(Cow<'a, [SftSent<'a>]>),

    #[event_version("1.0.0")]
    #[serde(rename = "sft_received")]
    Received(Cow<'a, [SftReceived<'a>]>),

    #[event_version("1.0.0")]
    #[serde(rename = "sft_burned")]
    Burned(Cow<'a, [SftBurned<'a>]>),
}

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct SftMinted<'a> {
    // TODO: include minter_id everywhere?
    pub owner_id: Cow<'a, AccountIdRef>,

    #[serde_as(as = "DisplayFromStr")]
    pub amount: u128,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<Cow<'a, str>>,
}

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct SftSent<'a> {
    pub sender_id: Cow<'a, AccountIdRef>,

    pub receiver_id: Cow<'a, AccountIdRef>,

    #[serde_as(as = "DisplayFromStr")]
    pub amount: u128,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<Cow<'a, str>>,
}

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct SftReceived<'a> {
    pub sender_id: Cow<'a, AccountIdRef>,

    pub receiver_id: Cow<'a, AccountIdRef>,

    #[serde_as(as = "DisplayFromStr")]
    pub amount: u128,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<Cow<'a, str>>,
}

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct SftBurned<'a> {
    pub owner_id: Cow<'a, AccountIdRef>,

    #[serde_as(as = "DisplayFromStr")]
    pub amount: u128,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<Cow<'a, str>>,
}
