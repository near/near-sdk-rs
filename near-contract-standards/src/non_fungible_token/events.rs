//! Standard for nep171 (Non-Fungible Token) events.
//!
//! These events will be picked up by the NEAR indexer.
//!
//! <https://github.com/near/NEPs/blob/69f76c6c78c2ebf05d856347c9c98ae48ad84ebd/specs/Standards/NonFungibleToken/Event.md>
//!
//! This is an extension of the events format (nep-297):
//! <https://github.com/near/NEPs/blob/master/specs/Standards/EventsFormat.md>
//!
//! The three events in this standard are [`NftMintData`], [`NftTransferData`], and [`NftBurnData`].
//!
//! These events can be logged by calling `.emit()` on them if a single event, or calling
//! [`emit_nft_mints`], [`emit_nft_transfers`], or [`emit_nft_burns`] respectively.

use crate::event::NearEvent;
use near_sdk::AccountId;
use serde::Serialize;
use serde_with::skip_serializing_none;

#[derive(Serialize, Debug)]
pub(crate) struct Nep171Event<'a> {
    version: &'static str,
    #[serde(flatten)]
    event_kind: Nep171EventKind<'a>,
}

#[derive(Serialize, Debug)]
#[serde(tag = "event", content = "data")]
#[serde(rename_all = "snake_case")]
#[allow(clippy::enum_variant_names)]
enum Nep171EventKind<'a> {
    NftMint(&'a [NftMintData<'a>]),
    NftTransfer(&'a [NftTransferData<'a>]),
    NftBurn(&'a [NftBurnData<'a>]),
}

/// Data to log for an NFT mint event. To log this event, call [`.emit()`](NftMintData::emit).
#[skip_serializing_none]
#[must_use]
#[derive(Serialize, Debug)]
pub struct NftMintData<'a> {
    pub owner_id: &'a AccountId,
    pub token_ids: &'a [&'a str],
    pub memo: Option<&'a str>,
}

impl NftMintData<'_> {
    /// Logs the event to the host. This is required to ensure that the event is triggered
    /// and to consume the event.
    pub fn emit(self) {
        emit_nft_mints(&[self])
    }
}

/// Data to log for an NFT transfer event. To log this event,
/// call [`.emit()`](NftTransferData::emit).
#[skip_serializing_none]
#[must_use]
#[derive(Serialize, Debug)]
pub struct NftTransferData<'a> {
    pub old_owner_id: &'a AccountId,
    pub new_owner_id: &'a AccountId,
    pub token_ids: &'a [&'a str],
    pub authorized_id: Option<&'a AccountId>,
    pub memo: Option<&'a str>,
}

impl NftTransferData<'_> {
    /// Logs the event to the host. This is required to ensure that the event is triggered
    /// and to consume the event.
    pub fn emit(self) {
        emit_nft_transfers(&[self])
    }
}

/// Data to log for an NFT burn event. To log this event, call [`.emit()`](NftBurnData::emit).
#[skip_serializing_none]
#[must_use]
#[derive(Serialize, Debug)]
pub struct NftBurnData<'a> {
    pub owner_id: &'a AccountId,
    pub token_ids: &'a [&'a str],
    pub authorized_id: Option<&'a AccountId>,
    pub memo: Option<&'a str>,
}

impl NftBurnData<'_> {
    /// Logs the event to the host. This is required to ensure that the event is triggered
    /// and to consume the event.
    pub fn emit(self) {
        emit_nft_burns(&[self])
    }
}

fn new_171<'a>(version: &'static str, event_kind: Nep171EventKind<'a>) -> NearEvent<'a> {
    NearEvent::Nep171(Nep171Event { version, event_kind })
}

fn new_171_v1(event_kind: Nep171EventKind) -> NearEvent {
    new_171("1.0.0", event_kind)
}

/// Emits an nft burn event, through [`env::log_str`](near_sdk::env::log_str),
/// where each [`NftBurnData`] represents the data of each burn.
pub fn emit_nft_burns<'a>(data: &'a [NftBurnData<'a>]) {
    new_171_v1(Nep171EventKind::NftBurn(data)).emit()
}

/// Emits an nft transfer event, through [`env::log_str`](near_sdk::env::log_str),
/// where each [`NftTransferData`] represents the data of each transfer.
pub fn emit_nft_transfers<'a>(data: &'a [NftTransferData<'a>]) {
    new_171_v1(Nep171EventKind::NftTransfer(data)).emit()
}

/// Emits an nft mint event, through [`env::log_str`](near_sdk::env::log_str),
/// where each [`NftMintData`] represents the data of each mint.
pub fn emit_nft_mints<'a>(data: &'a [NftMintData<'a>]) {
    new_171_v1(Nep171EventKind::NftMint(data)).emit()
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::{test_utils, AccountId};

    fn bob() -> AccountId {
        AccountId::new_unchecked("bob".to_string())
    }

    fn alice() -> AccountId {
        AccountId::new_unchecked("alice".to_string())
    }

    #[test]
    fn nft_mint() {
        let owner_id = &bob();
        let token_ids = &["0", "1"];
        NftMintData { owner_id, token_ids, memo: None }.emit();
        assert_eq!(
            test_utils::get_logs()[0],
            r#"EVENT_JSON:{"standard":"nep171","version":"1.0.0","event":"nft_mint","data":[{"owner_id":"bob","token_ids":["0","1"]}]}"#
        );
    }

    #[test]
    fn nft_mints() {
        let owner_id = &bob();
        let token_ids = &["0", "1"];
        let mint_log = NftMintData { owner_id, token_ids, memo: None };
        emit_nft_mints(&[
            mint_log,
            NftMintData { owner_id: &alice(), token_ids: &["2", "3"], memo: Some("has memo") },
        ]);
        assert_eq!(
            test_utils::get_logs()[0],
            r#"EVENT_JSON:{"standard":"nep171","version":"1.0.0","event":"nft_mint","data":[{"owner_id":"bob","token_ids":["0","1"]},{"owner_id":"alice","token_ids":["2","3"],"memo":"has memo"}]}"#
        );
    }

    #[test]
    fn nft_burn() {
        let owner_id = &bob();
        let token_ids = &["0", "1"];
        NftBurnData { owner_id, token_ids, authorized_id: None, memo: None }.emit();
        assert_eq!(
            test_utils::get_logs()[0],
            r#"EVENT_JSON:{"standard":"nep171","version":"1.0.0","event":"nft_burn","data":[{"owner_id":"bob","token_ids":["0","1"]}]}"#
        );
    }

    #[test]
    fn nft_burns() {
        let owner_id = &bob();
        let token_ids = &["0", "1"];
        emit_nft_burns(&[
            NftBurnData {
                owner_id: &alice(),
                token_ids: &["2", "3"],
                authorized_id: Some(&bob()),
                memo: Some("has memo"),
            },
            NftBurnData { owner_id, token_ids, authorized_id: None, memo: None },
        ]);
        assert_eq!(
            test_utils::get_logs()[0],
            r#"EVENT_JSON:{"standard":"nep171","version":"1.0.0","event":"nft_burn","data":[{"owner_id":"alice","token_ids":["2","3"],"authorized_id":"bob","memo":"has memo"},{"owner_id":"bob","token_ids":["0","1"]}]}"#
        );
    }

    #[test]
    fn nft_transfer() {
        let old_owner_id = &bob();
        let new_owner_id = &alice();
        let token_ids = &["0", "1"];
        NftTransferData { old_owner_id, new_owner_id, token_ids, authorized_id: None, memo: None }
            .emit();
        assert_eq!(
            test_utils::get_logs()[0],
            r#"EVENT_JSON:{"standard":"nep171","version":"1.0.0","event":"nft_transfer","data":[{"old_owner_id":"bob","new_owner_id":"alice","token_ids":["0","1"]}]}"#
        );
    }

    #[test]
    fn nft_transfers() {
        let old_owner_id = &bob();
        let new_owner_id = &alice();
        let token_ids = &["0", "1"];
        emit_nft_transfers(&[
            NftTransferData {
                old_owner_id: &alice(),
                new_owner_id: &bob(),
                token_ids: &["2", "3"],
                authorized_id: Some(&bob()),
                memo: Some("has memo"),
            },
            NftTransferData {
                old_owner_id,
                new_owner_id,
                token_ids,
                authorized_id: None,
                memo: None,
            },
        ]);
        assert_eq!(
            test_utils::get_logs()[0],
            r#"EVENT_JSON:{"standard":"nep171","version":"1.0.0","event":"nft_transfer","data":[{"old_owner_id":"alice","new_owner_id":"bob","token_ids":["2","3"],"authorized_id":"bob","memo":"has memo"},{"old_owner_id":"bob","new_owner_id":"alice","token_ids":["0","1"]}]}"#
        );
    }
}
