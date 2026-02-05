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

/// Maximum total log length allowed by NEAR protocol (16KB).
pub const TOTAL_LOG_LENGTH_LIMIT: usize = 16 * 1024;

/// Standard memo used for refund events.
pub const REFUND_MEMO: &str = "refund";

/// Extra bytes overhead when adding a memo field to an event.
/// This accounts for the JSON structure: `,"memo":"refund"`
pub const REFUND_MEMO_EXTRA_BYTES: usize = 10 + REFUND_MEMO.len(); // `,"memo":"` + `refund` + `"`

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

    /// Calculates the approximate log length if this event were emitted.
    /// This is useful for checking if a refund event would exceed the log limit.
    pub fn estimate_log_length(&self) -> usize {
        // Calculate base event overhead
        let base_overhead = r#"EVENT_JSON:{"standard":"nep245","version":"1.0.0","event":"mt_transfer","data":[{}]}"#.len();

        // Calculate this event's content length
        let mut content_len = 0;

        // old_owner_id: "old_owner_id":"..."
        content_len += 15 + self.old_owner_id.len();

        // new_owner_id: ,"new_owner_id":"..."
        content_len += 17 + self.new_owner_id.len();

        // token_ids: ,"token_ids":[...]
        content_len += 14;
        for (i, token_id) in self.token_ids.iter().enumerate() {
            if i > 0 {
                content_len += 1; // comma
            }
            content_len += 2 + token_id.len(); // quotes + content
        }

        // amounts: ,"amounts":[...]
        content_len += 12;
        for (i, amount) in self.amounts.iter().enumerate() {
            if i > 0 {
                content_len += 1; // comma
            }
            content_len += 2 + amount.0.to_string().len(); // quotes + number as string
        }

        // authorized_id if present
        if let Some(auth_id) = &self.authorized_id {
            content_len += 18 + auth_id.len(); // ,"authorized_id":"..."
        }

        // memo if present
        if let Some(memo) = &self.memo {
            content_len += 10 + memo.len(); // ,"memo":"..."
        }

        base_overhead + content_len
    }

    /// Calculates the extra bytes needed if a refund memo would be added to events without memos.
    /// Returns the overhead that would be added when converting this to a refund event.
    pub fn refund_overhead(&self) -> usize {
        match self.memo {
            None => REFUND_MEMO_EXTRA_BYTES,
            Some(m) if m.len() < REFUND_MEMO.len() => REFUND_MEMO.len() - m.len(),
            Some(_) => 0,
        }
    }

    /// Checks if this transfer event (when used as a refund) would fit within the log limit.
    /// This accounts for the additional memo field that would be added during refunds.
    pub fn would_refund_fit_in_log(&self) -> bool {
        self.estimate_log_length() + self.refund_overhead() <= TOTAL_LOG_LENGTH_LIMIT
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
