use std::borrow::Cow;

use crate::env;
use crate::serde::Serialize;
use crate::serde_json;

/// Representation of an event formatted according to [NEP-297](https://github.com/near/NEPs/blob/master/neps/nep-0297.md).
///
/// `standard`, `version`, and `event` are required by the specification. The `data` field is optional
/// and omitted during serialization when it is `None`.
#[derive(Debug, Serialize)]
#[serde(crate = "crate::serde")]
pub struct Nep297Event<'a, T>
where
    T: Serialize,
{
    pub standard: Cow<'a, str>,
    pub version: Cow<'a, str>,
    pub event: Cow<'a, str>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

impl<'a, T> Nep297Event<'a, T>
where
    T: Serialize,
{
    /// Returns the event as a [`serde_json::Value`].
    ///
    /// This is kept, alongside [`Self::to_json_value`], for callers that expect the shorter method
    /// name when working with NEP-297 helpers.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_else(|_| env::abort())
    }

    /// Returns the event serialized as a JSON string.
    pub fn to_json_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| env::abort())
    }

    /// Returns the NEAR log line that should be emitted (`EVENT_JSON:{json}`).
    pub fn to_event_log(&self) -> String {
        format!("EVENT_JSON:{}", self.to_json_string())
    }
}

/// Convenience trait for NEP-297-compatible event types.
///
/// Implementers supply the required metadata fields and optionally provide the event payload.
/// The default implementations expose helpers for logging and JSON conversion.
pub trait AsNep297Event: Serialize {
    /// Returns the event standard (e.g. `"nep171"`).
    fn standard(&self) -> Cow<'_, str>;

    /// Returns the event standard version (e.g. `"1.0.0"`).
    fn version(&self) -> Cow<'_, str>;

    /// Returns the event name (e.g. `"nft_mint"`).
    fn event(&self) -> Cow<'_, str>;

    /// Optionally returns the payload that will appear under the `data` field.
    ///
    /// By default events do not contain data.
    fn data(&self) -> Option<serde_json::Value> {
        None
    }

    /// Converts the event into a [`Nep297Event`] representation.
    fn to_nep297_event(&self) -> Nep297Event<'_, serde_json::Value> {
        Nep297Event {
            standard: self.standard(),
            version: self.version(),
            event: self.event(),
            data: self.data(),
        }
    }
}
