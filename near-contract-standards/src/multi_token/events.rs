//! Standard for nep245 (Multi Token) events.
//!
//! These events will be picked up by the NEAR indexer.
//!
//! <https://github.com/near/NEPs/blob/master/neps/nep-0245.md>
//!
//! This is an extension of the events format (nep-297):
//! <https://github.com/near/NEPs/blob/master/specs/Standards/EventsFormat.md>
//!
//! The three events in this standard are [`MtMint`], [`MtTransfer`], and [`MtBurn`].
//!
//! These events can be logged by calling `.emit()` on them if a single event, or calling
//! [`MtMint::emit_many`], [`MtTransfer::emit_many`],
//! or [`MtBurn::emit_many`] respectively.

use crate::event::NearEvent;
use near_sdk::json_types::U128;
use near_sdk::serde::Serialize;
use near_sdk::AccountIdRef;

/// Data to log for an MT mint event. To log this event, call [`.emit()`](MtMint::emit).
#[must_use]
#[derive(Serialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct MtMint<'a> {
    pub owner_id: &'a AccountIdRef,
    pub token_ids: &'a [&'a str],
    pub amounts: &'a [U128],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<&'a str>,
}

impl MtMint<'_> {
    /// Logs the event to the host. This is required to ensure that the event is triggered
    /// and to consume the event.
    pub fn emit(self) {
        Self::emit_many(&[self])
    }

    /// Emits an mt mint event, through [`env::log_str`](near_sdk::env::log_str),
    /// where each [`MtMint`] represents the data of each mint.
    pub fn emit_many(data: &[MtMint<'_>]) {
        new_245_v1(Nep245EventKind::MtMint(data)).emit()
    }
}

/// Data to log for an MT transfer event. To log this event,
/// call [`.emit()`](MtTransfer::emit).
#[must_use]
#[derive(Serialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct MtTransfer<'a> {
    pub old_owner_id: &'a AccountIdRef,
    pub new_owner_id: &'a AccountIdRef,
    pub token_ids: &'a [&'a str],
    pub amounts: &'a [U128],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorized_id: Option<&'a AccountIdRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<&'a str>,
}

impl MtTransfer<'_> {
    /// Logs the event to the host. This is required to ensure that the event is triggered
    /// and to consume the event.
    pub fn emit(self) {
        Self::emit_many(&[self])
    }

    /// Emits an mt transfer event, through [`env::log_str`](near_sdk::env::log_str),
    /// where each [`MtTransfer`] represents the data of each transfer.
    pub fn emit_many(data: &[MtTransfer<'_>]) {
        new_245_v1(Nep245EventKind::MtTransfer(data)).emit()
    }
}

/// Data to log for an MT burn event. To log this event, call [`.emit()`](MtBurn::emit).
#[must_use]
#[derive(Serialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct MtBurn<'a> {
    pub owner_id: &'a AccountIdRef,
    pub token_ids: &'a [&'a str],
    pub amounts: &'a [U128],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorized_id: Option<&'a AccountIdRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<&'a str>,
}

impl MtBurn<'_> {
    /// Logs the event to the host. This is required to ensure that the event is triggered
    /// and to consume the event.
    pub fn emit(self) {
        Self::emit_many(&[self])
    }

    /// Emits an mt burn event, through [`env::log_str`](near_sdk::env::log_str),
    /// where each [`MtBurn`] represents the data of each burn.
    pub fn emit_many<'a>(data: &'a [MtBurn<'a>]) {
        new_245_v1(Nep245EventKind::MtBurn(data)).emit()
    }
}

#[derive(Serialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub(crate) struct Nep245Event<'a> {
    version: &'static str,
    #[serde(flatten)]
    event_kind: Nep245EventKind<'a>,
}

#[derive(Serialize, Debug)]
#[serde(crate = "near_sdk::serde")]
#[serde(tag = "event", content = "data")]
#[serde(rename_all = "snake_case")]
#[allow(clippy::enum_variant_names)]
enum Nep245EventKind<'a> {
    MtMint(&'a [MtMint<'a>]),
    MtTransfer(&'a [MtTransfer<'a>]),
    MtBurn(&'a [MtBurn<'a>]),
}

fn new_245<'a>(version: &'static str, event_kind: Nep245EventKind<'a>) -> NearEvent<'a> {
    NearEvent::Nep245(Nep245Event { version, event_kind })
}

fn new_245_v1(event_kind: Nep245EventKind) -> NearEvent {
    new_245("1.0.0", event_kind)
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils;

    #[test]
    fn mt_mint() {
        let owner_id = AccountIdRef::new_or_panic("bob");
        let token_ids = &["0", "1"];
        let amounts = &[U128(1), U128(100)];
        MtMint { owner_id, token_ids, amounts, memo: None }.emit();
        assert_eq!(
            test_utils::get_logs()[0],
            r#"EVENT_JSON:{"standard":"nep245","version":"1.0.0","event":"mt_mint","data":[{"owner_id":"bob","token_ids":["0","1"],"amounts":["1","100"]}]}"#
        );
    }

    #[test]
    fn mt_mints() {
        let owner_id = AccountIdRef::new_or_panic("bob");
        let token_ids = &["0", "1"];
        let amounts = &[U128(1), U128(100)];
        let mint_log = MtMint { owner_id, token_ids, amounts, memo: None };
        MtMint::emit_many(&[
            mint_log,
            MtMint {
                owner_id: AccountIdRef::new_or_panic("alice"),
                token_ids: &["2", "3"],
                amounts: &[U128(1), U128(50)],
                memo: Some("has memo"),
            },
        ]);
        assert_eq!(
            test_utils::get_logs()[0],
            r#"EVENT_JSON:{"standard":"nep245","version":"1.0.0","event":"mt_mint","data":[{"owner_id":"bob","token_ids":["0","1"],"amounts":["1","100"]},{"owner_id":"alice","token_ids":["2","3"],"amounts":["1","50"],"memo":"has memo"}]}"#
        );
    }

    #[test]
    fn mt_burn() {
        let owner_id = AccountIdRef::new_or_panic("bob");
        let token_ids = &["0", "1"];
        let amounts = &[U128(1), U128(100)];
        MtBurn { owner_id, token_ids, amounts, authorized_id: None, memo: None }.emit();
        assert_eq!(
            test_utils::get_logs()[0],
            r#"EVENT_JSON:{"standard":"nep245","version":"1.0.0","event":"mt_burn","data":[{"owner_id":"bob","token_ids":["0","1"],"amounts":["1","100"]}]}"#
        );
    }

    #[test]
    fn mt_burns() {
        let owner_id = AccountIdRef::new_or_panic("bob");
        let token_ids = &["0", "1"];
        let amounts = &[U128(1), U128(100)];
        MtBurn::emit_many(&[
            MtBurn {
                owner_id: AccountIdRef::new_or_panic("alice"),
                token_ids: &["2", "3"],
                amounts: &[U128(1), U128(50)],
                authorized_id: Some(AccountIdRef::new_or_panic("bob")),
                memo: Some("has memo"),
            },
            MtBurn { owner_id, token_ids, amounts, authorized_id: None, memo: None },
        ]);
        assert_eq!(
            test_utils::get_logs()[0],
            r#"EVENT_JSON:{"standard":"nep245","version":"1.0.0","event":"mt_burn","data":[{"owner_id":"alice","token_ids":["2","3"],"amounts":["1","50"],"authorized_id":"bob","memo":"has memo"},{"owner_id":"bob","token_ids":["0","1"],"amounts":["1","100"]}]}"#
        );
    }

    #[test]
    fn mt_transfer() {
        let old_owner_id = AccountIdRef::new_or_panic("bob");
        let new_owner_id = AccountIdRef::new_or_panic("alice");
        let token_ids = &["0", "1"];
        let amounts = &[U128(1), U128(100)];
        MtTransfer {
            old_owner_id,
            new_owner_id,
            token_ids,
            amounts,
            authorized_id: None,
            memo: None,
        }
        .emit();
        assert_eq!(
            test_utils::get_logs()[0],
            r#"EVENT_JSON:{"standard":"nep245","version":"1.0.0","event":"mt_transfer","data":[{"old_owner_id":"bob","new_owner_id":"alice","token_ids":["0","1"],"amounts":["1","100"]}]}"#
        );
    }

    #[test]
    fn mt_transfers() {
        let old_owner_id = AccountIdRef::new_or_panic("bob");
        let new_owner_id = AccountIdRef::new_or_panic("alice");
        let token_ids = &["0", "1"];
        let amounts = &[U128(1), U128(100)];
        MtTransfer::emit_many(&[
            MtTransfer {
                old_owner_id: AccountIdRef::new_or_panic("alice"),
                new_owner_id: AccountIdRef::new_or_panic("bob"),
                token_ids: &["2", "3"],
                amounts: &[U128(1), U128(50)],
                authorized_id: Some(AccountIdRef::new_or_panic("bob")),
                memo: Some("has memo"),
            },
            MtTransfer {
                old_owner_id,
                new_owner_id,
                token_ids,
                amounts,
                authorized_id: None,
                memo: None,
            },
        ]);
        assert_eq!(
            test_utils::get_logs()[0],
            r#"EVENT_JSON:{"standard":"nep245","version":"1.0.0","event":"mt_transfer","data":[{"old_owner_id":"alice","new_owner_id":"bob","token_ids":["2","3"],"amounts":["1","50"],"authorized_id":"bob","memo":"has memo"},{"old_owner_id":"bob","new_owner_id":"alice","token_ids":["0","1"],"amounts":["1","100"]}]}"#
        );
    }
}
