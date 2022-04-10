use crate::event::NearEvent;
use near_sdk::AccountId;
use serde::Serialize;

/// Data to log for an Multi-token mint event. To log this event, call [`.emit()`](MtMint::emit).
#[must_use]
#[derive(Serialize, Debug, Clone)]
pub struct MtMint<'a> {
    pub owner_id: &'a AccountId,
    pub token_ids: &'a [&'a str],
    pub amounts: &'a [&'a str],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<&'a str>,
}

impl MtMint<'_> {
    pub fn emit(self) {
        Self::emit_many(&[self])
    }

    pub fn emit_many(data: &[MtMint<'_>]) {
        new_245_v1(Nep245EventKind::MtMint(data)).emit()
    }
}

#[must_use]
#[derive(Serialize, Debug, Clone)]
pub struct MtTransfer<'a> {
    pub old_owner_id: &'a AccountId,
    pub new_owner_id: &'a AccountId,
    pub token_ids: &'a [&'a str],
    pub amounts: &'a [&'a str],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorized_id: Option<&'a AccountId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<&'a str>,
}

impl MtTransfer<'_> {
    pub fn emit(self) {
        Self::emit_many(&[self])
    }

    pub fn emit_many(data: &[MtTransfer<'_>]) {
        new_245_v1(Nep245EventKind::MtTransfer(data)).emit()
    }
}

#[must_use]
#[derive(Serialize, Debug, Clone)]
pub struct MtBurn<'a> {
    pub owner_id: &'a AccountId,
    pub token_ids: &'a [&'a str],
    pub amounts: &'a [&'a str],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorized_id: Option<&'a AccountId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<&'a str>,
}

impl MtBurn<'_> {
    pub fn emit(self) {
        Self::emit_many(&[self])
    }

    pub fn emit_many(data: &[MtBurn<'_>]) {
        new_245_v1(Nep245EventKind::MtBurn(data)).emit()
    }
}

#[derive(Serialize, Debug)]
pub(crate) struct Nep245Event<'a> {
    version: &'static str,
    #[serde(flatten)]
    event_kind: Nep245EventKind<'a>,
}

#[derive(Serialize, Debug)]
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
    use near_sdk::{test_utils, AccountId};

    fn bob() -> AccountId {
        AccountId::new_unchecked("bob".to_string())
    }

    fn alice() -> AccountId {
        AccountId::new_unchecked("alice".to_string())
    }

    #[test]
    fn mt_mint() {
        let owner_id = &bob();
        let token_ids = &["0", "1"];
        let amounts = &["1000", "90000"];
        MtMint { owner_id, token_ids, amounts, memo: None }.emit();
        assert_eq!(
            test_utils::get_logs()[0],
            r#"EVENT_JSON:{"standard":"nep245","version":"1.0.0","event":"mt_mint","data":[{"owner_id":"bob","token_ids":["0","1"],"amounts":["1000","90000"]}]}"#
        );
    }

    #[test]
    fn mt_mints() {
        MtMint::emit_many(&[
            MtMint {
                owner_id: &alice(),
                token_ids: &["0", "1"],
                amounts: &["1000", "90000"],
                memo: None,
            },
            MtMint { owner_id: &bob(), token_ids: &["2"], amounts: &["1"], memo: Some("has memo") },
        ]);
        assert_eq!(
            test_utils::get_logs()[0],
            r#"EVENT_JSON:{"standard":"nep245","version":"1.0.0","event":"mt_mint","data":[{"owner_id":"alice","token_ids":["0","1"],"amounts":["1000","90000"]},{"owner_id":"bob","token_ids":["2"],"amounts":["1"],"memo":"has memo"}]}"#
        );
    }

    #[test]
    fn mt_burn() {
        let owner_id = &bob();
        let token_ids = &["0", "1"];
        let amounts = &["20", "40"];
        let authorized_id = &alice();
        MtBurn { owner_id, token_ids, amounts, authorized_id: Some(authorized_id), memo: None }
            .emit();
        assert_eq!(
            test_utils::get_logs()[0],
            r#"EVENT_JSON:{"standard":"nep245","version":"1.0.0","event":"mt_burn","data":[{"owner_id":"bob","token_ids":["0","1"],"amounts":["20","40"],"authorized_id":"alice"}]}"#
        );
    }

    #[test]
    fn mt_burns() {
        MtBurn::emit_many(&[
            MtBurn {
                owner_id: &alice(),
                token_ids: &["0", "1"],
                amounts: &["1000", "90000"],
                authorized_id: None,
                memo: None,
            },
            MtBurn {
                owner_id: &bob(),
                token_ids: &["2"],
                amounts: &["1"],
                authorized_id: Some(&alice()),
                memo: Some("has memo"),
            },
        ]);
        assert_eq!(
            test_utils::get_logs()[0],
            r#"EVENT_JSON:{"standard":"nep245","version":"1.0.0","event":"mt_burn","data":[{"owner_id":"alice","token_ids":["0","1"],"amounts":["1000","90000"]},{"owner_id":"bob","token_ids":["2"],"amounts":["1"],"authorized_id":"alice","memo":"has memo"}]}"#
        );
    }

    #[test]
    fn mt_transfer() {
        let old_owner_id = &bob();
        let new_owner_id = &alice();
        let token_ids = &["0", "1"];
        let amounts = &["48", "99"];
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
            r#"EVENT_JSON:{"standard":"nep245","version":"1.0.0","event":"mt_transfer","data":[{"old_owner_id":"bob","new_owner_id":"alice","token_ids":["0","1"],"amounts":["48","99"]}]}"#
        );
    }

    #[test]
    fn mt_transfers() {
        MtTransfer::emit_many(&[
            MtTransfer {
                old_owner_id: &alice(),
                new_owner_id: &bob(),
                token_ids: &["0", "1"],
                amounts: &["48", "99"],
                authorized_id: None,
                memo: Some("has memo"),
            },
            MtTransfer {
                old_owner_id: &bob(),
                new_owner_id: &alice(),
                token_ids: &["1", "2"],
                amounts: &["1", "99"],
                authorized_id: Some(&bob()),
                memo: None,
            },
        ]);
        assert_eq!(
            test_utils::get_logs()[0],
            concat!(
                r#"EVENT_JSON:{"standard":"nep245","version":"1.0.0","event":"mt_transfer","data":["#,
                r#"{"old_owner_id":"alice","new_owner_id":"bob","token_ids":["0","1"],"amounts":["48","99"],"memo":"has memo"},"#,
                r#"{"old_owner_id":"bob","new_owner_id":"alice","token_ids":["1","2"],"amounts":["1","99"],"authorized_id":"bob"}]}"#
            )
        );
    }
}
