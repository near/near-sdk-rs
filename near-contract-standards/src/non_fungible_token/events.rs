//! Standard for nep171 (Non-Fungible Token) events.
//!
//! These events will be picked up by the NEAR indexer.
//!
//! <https://github.com/near/NEPs/blob/69f76c6c78c2ebf05d856347c9c98ae48ad84ebd/specs/Standards/NonFungibleToken/Event.md>
//!
//! This is an extension of the events format (nep-297):
//! <https://github.com/near/NEPs/blob/master/specs/Standards/EventsFormat.md>
//!
//! The three events in this standard are [`NftMint`], [`NftTransfer`], and [`NftBurn`].
//!
//! These events can be logged by calling `.emit()` on them if a single event, or calling
//! [`NftMint::emit_many`], [`NftTransfer::emit_many`],
//! or [`NftBurn::emit_many`] respectively.

use crate::event::NearEvent;
use near_sdk::serde::Serialize;
use near_sdk::AccountIdRef;

/// Data to log for an NFT mint event. To log this event, call [`.emit()`](NftMint::emit).
#[must_use]
#[derive(Serialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct NftMint<'a> {
    pub owner_id: &'a AccountIdRef,
    pub token_ids: &'a [&'a str],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<&'a str>,
}

impl NftMint<'_> {
    /// Logs the event to the host. This is required to ensure that the event is triggered
    /// and to consume the event.
    pub fn emit(self) {
        Self::emit_many(&[self])
    }

    /// Emits an nft mint event, through [`env::log_str`](near_sdk::env::log_str),
    /// where each [`NftMint`] represents the data of each mint.
    pub fn emit_many(data: &[NftMint<'_>]) {
        new_171_v1(Nep171EventKind::NftMint(data)).emit()
    }
}

/// Data to log for an NFT transfer event. To log this event,
/// call [`.emit()`](NftTransfer::emit).
#[must_use]
#[derive(Serialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct NftTransfer<'a> {
    pub old_owner_id: &'a AccountIdRef,
    pub new_owner_id: &'a AccountIdRef,
    pub token_ids: &'a [&'a str],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorized_id: Option<&'a AccountIdRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<&'a str>,
}

impl NftTransfer<'_> {
    /// Logs the event to the host. This is required to ensure that the event is triggered
    /// and to consume the event.
    pub fn emit(self) {
        Self::emit_many(&[self])
    }

    /// Emits an nft transfer event, through [`env::log_str`](near_sdk::env::log_str),
    /// where each [`NftTransfer`] represents the data of each transfer.
    pub fn emit_many(data: &[NftTransfer<'_>]) {
        new_171_v1(Nep171EventKind::NftTransfer(data)).emit()
    }
}

/// Data to log for an NFT burn event. To log this event, call [`.emit()`](NftBurn::emit).
#[must_use]
#[derive(Serialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct NftBurn<'a> {
    pub owner_id: &'a AccountIdRef,
    pub token_ids: &'a [&'a str],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorized_id: Option<&'a AccountIdRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<&'a str>,
}

impl NftBurn<'_> {
    /// Logs the event to the host. This is required to ensure that the event is triggered
    /// and to consume the event.
    pub fn emit(self) {
        Self::emit_many(&[self])
    }

    /// Emits an nft burn event, through [`env::log_str`](near_sdk::env::log_str),
    /// where each [`NftBurn`] represents the data of each burn.
    pub fn emit_many<'a>(data: &'a [NftBurn<'a>]) {
        new_171_v1(Nep171EventKind::NftBurn(data)).emit()
    }
}

#[derive(Serialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub(crate) struct Nep171Event<'a> {
    version: &'static str,
    #[serde(flatten)]
    event_kind: Nep171EventKind<'a>,
}

#[derive(Serialize, Debug)]
#[serde(crate = "near_sdk::serde")]
#[serde(tag = "event", content = "data")]
#[serde(rename_all = "snake_case")]
#[allow(clippy::enum_variant_names)]
enum Nep171EventKind<'a> {
    NftMint(&'a [NftMint<'a>]),
    NftTransfer(&'a [NftTransfer<'a>]),
    NftBurn(&'a [NftBurn<'a>]),
}

fn new_171<'a>(version: &'static str, event_kind: Nep171EventKind<'a>) -> NearEvent<'a> {
    NearEvent::Nep171(Nep171Event { version, event_kind })
}

fn new_171_v1(event_kind: Nep171EventKind) -> NearEvent {
    new_171("1.0.0", event_kind)
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils;

    #[test]
    fn nft_mint() {
        let owner_id = AccountIdRef::new_or_panic("bob");
        let token_ids = &["0", "1"];
        NftMint { owner_id, token_ids, memo: None }.emit();
        assert_eq!(
            test_utils::get_logs()[0],
            r#"EVENT_JSON:{"standard":"nep171","version":"1.0.0","event":"nft_mint","data":[{"owner_id":"bob","token_ids":["0","1"]}]}"#
        );
    }

    #[test]
    fn nft_mints() {
        let owner_id = AccountIdRef::new_or_panic("bob");
        let token_ids = &["0", "1"];
        let mint_log = NftMint { owner_id, token_ids, memo: None };
        NftMint::emit_many(&[
            mint_log,
            NftMint {
                owner_id: AccountIdRef::new_or_panic("alice"),
                token_ids: &["2", "3"],
                memo: Some("has memo"),
            },
        ]);
        assert_eq!(
            test_utils::get_logs()[0],
            r#"EVENT_JSON:{"standard":"nep171","version":"1.0.0","event":"nft_mint","data":[{"owner_id":"bob","token_ids":["0","1"]},{"owner_id":"alice","token_ids":["2","3"],"memo":"has memo"}]}"#
        );
    }

    #[test]
    fn nft_burn() {
        let owner_id = AccountIdRef::new_or_panic("bob");
        let token_ids = &["0", "1"];
        NftBurn { owner_id, token_ids, authorized_id: None, memo: None }.emit();
        assert_eq!(
            test_utils::get_logs()[0],
            r#"EVENT_JSON:{"standard":"nep171","version":"1.0.0","event":"nft_burn","data":[{"owner_id":"bob","token_ids":["0","1"]}]}"#
        );
    }

    #[test]
    fn nft_burns() {
        let owner_id = AccountIdRef::new_or_panic("bob");
        let token_ids = &["0", "1"];
        NftBurn::emit_many(&[
            NftBurn {
                owner_id: AccountIdRef::new_or_panic("alice"),
                token_ids: &["2", "3"],
                authorized_id: Some(AccountIdRef::new_or_panic("bob")),
                memo: Some("has memo"),
            },
            NftBurn { owner_id, token_ids, authorized_id: None, memo: None },
        ]);
        assert_eq!(
            test_utils::get_logs()[0],
            r#"EVENT_JSON:{"standard":"nep171","version":"1.0.0","event":"nft_burn","data":[{"owner_id":"alice","token_ids":["2","3"],"authorized_id":"bob","memo":"has memo"},{"owner_id":"bob","token_ids":["0","1"]}]}"#
        );
    }

    #[test]
    fn nft_transfer() {
        let old_owner_id = AccountIdRef::new_or_panic("bob");
        let new_owner_id = AccountIdRef::new_or_panic("alice");
        let token_ids = &["0", "1"];
        NftTransfer { old_owner_id, new_owner_id, token_ids, authorized_id: None, memo: None }
            .emit();
        assert_eq!(
            test_utils::get_logs()[0],
            r#"EVENT_JSON:{"standard":"nep171","version":"1.0.0","event":"nft_transfer","data":[{"old_owner_id":"bob","new_owner_id":"alice","token_ids":["0","1"]}]}"#
        );
    }

    #[test]
    fn nft_transfers() {
        let old_owner_id = AccountIdRef::new_or_panic("bob");
        let new_owner_id = AccountIdRef::new_or_panic("alice");
        let token_ids = &["0", "1"];
        NftTransfer::emit_many(&[
            NftTransfer {
                old_owner_id: AccountIdRef::new_or_panic("alice"),
                new_owner_id: AccountIdRef::new_or_panic("bob"),
                token_ids: &["2", "3"],
                authorized_id: Some(AccountIdRef::new_or_panic("bob")),
                memo: Some("has memo"),
            },
            NftTransfer { old_owner_id, new_owner_id, token_ids, authorized_id: None, memo: None },
        ]);
        assert_eq!(
            test_utils::get_logs()[0],
            r#"EVENT_JSON:{"standard":"nep171","version":"1.0.0","event":"nft_transfer","data":[{"old_owner_id":"alice","new_owner_id":"bob","token_ids":["2","3"],"authorized_id":"bob","memo":"has memo"},{"old_owner_id":"bob","new_owner_id":"alice","token_ids":["0","1"]}]}"#
        );
    }
}
