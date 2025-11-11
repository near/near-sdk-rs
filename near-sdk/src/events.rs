use std::borrow::Cow;

use crate::env;
use crate::serde::Serialize;
use crate::serde_json;

/// Wraps an event payload with its [NEP-297](https://github.com/near/NEPs/blob/master/neps/nep-0297.md) metadata.
///
/// `standard`, `version`, and `event` map directly to the NEP-297 fields. The payload is stored by
/// reference under `data` so callers retain type safety and avoid eager serialization.
///
/// # Type Parameters
/// - `T`: Event payload that implements [`Serialize`]. The payload is borrowed so it can be
///   serialized lazily by helper methods or user code.
#[derive(Debug, Serialize)]
#[serde(crate = "crate::serde")]
pub struct Nep297Event<'a, T: Serialize> {
    standard: Cow<'a, str>,
    version: Cow<'a, str>,

    // NEP-297 expects the payload to contribute the `event` key, so skip our copy to prevent duplicates.
    #[serde(skip_serializing)]
    event: Cow<'a, str>,

    // Flatten the payload so its `event`/`data` keys appear alongside the NEP-297 metadata.
    #[serde(flatten)]
    data: &'a T,
}

impl<'a, T> Nep297Event<'a, T>
where
    T: Serialize,
{
    /// Constructs a new NEP-297 wrapper from the provided metadata and payload.
    pub fn new<S, V, E>(standard: S, version: V, event: E, data: &'a T) -> Self
    where
        S: Into<Cow<'a, str>>,
        V: Into<Cow<'a, str>>,
        E: Into<Cow<'a, str>>,
    {
        Self { standard: standard.into(), version: version.into(), event: event.into(), data }
    }

    /// Returns the NEP-297 standard identifier associated with this event.
    pub fn standard(&self) -> &str {
        &self.standard
    }

    /// Returns the semantic version string declared for this event.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Returns the snake-cased event name used in the emitted log.
    pub fn event(&self) -> &str {
        &self.event
    }

    /// Serializes the payload and extracts the value stored under the `data` key, if any.
    pub fn data(&self) -> Option<serde_json::Value> {
        match serde_json::to_value(self).unwrap_or_else(|_| env::abort()) {
            serde_json::Value::Object(mut map) => map.remove("data"),
            _ => None,
        }
    }

    /// Returns the event as a [`serde_json::Value`].
    ///
    /// Kept for existing callers that expect the shorter `to_json` spelling when working with
    /// NEP-297 helpers.
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

/// Helper trait implemented by events that expose a NEP-297 representation.
pub trait AsNep297Event: Serialize + Sized {
    /// Converts the event into a [`Nep297Event`] representation.
    ///
    /// Wraps the event with its NEP-297 metadata. The payload is borrowed so it can be serialized
    /// on demand without cloning.
    fn to_nep297_event(&self) -> Nep297Event<'_, Self>;
}
