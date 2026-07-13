//! Compile-level regression test for `#[near(crate = "...")]` (near/near-sdk-rs#1059).
//!
//! Simulates a downstream wrapper crate (like `defuse_wallet` in `near/intents`, which wraps
//! `#[near]` in its own `wallet!` macro) that re-exports `near-sdk` under a renamed dependency
//! and a nested module path, so that consumers of the wrapper never need `near-sdk` as a direct
//! dependency with a matching version.
//!
//! `near-sdk` is renamed to `renamed_near_sdk` in `Cargo.toml`, and re-exported one module level
//! deep below that -- both details a naive implementation could get away with hardcoding around.

pub mod vendor {
    pub use renamed_near_sdk as near_sdk;
}

// `#[near]` derives `BorshSerialize`/`BorshDeserialize` itself (via the resolved `crate = "..."`
// path below), so only `Debug`/`PanicOnDefault` need to be added here. `PanicOnDefault` also
// takes its own `crate` argument (via the `#[panic_on_default(...)]` helper attribute), which
// gets exercised here too.
#[vendor::near_sdk::near(contract_state, crate = "crate::vendor::near_sdk")]
#[derive(Debug, vendor::near_sdk::PanicOnDefault)]
#[panic_on_default(crate = "crate::vendor::near_sdk")]
pub struct Counter {
    value: u64,
}

#[vendor::near_sdk::near(crate = "crate::vendor::near_sdk")]
impl Counter {
    #[init]
    pub fn new() -> Self {
        Self { value: 0 }
    }

    pub fn increment(&mut self) -> u64 {
        self.value += 1;
        self.value
    }

    pub fn get(&self) -> u64 {
        self.value
    }
}

// NEP-297 events go through a separate `near_events`/`EventMetadata` codegen path (not
// `near_bindgen`), which needed its own fix to accept `crate = "..."` -- see
// near-sdk-rs#1059 review discussion. Covered here so a regression doesn't slip back in.
#[vendor::near_sdk::near(event_json(standard = "nep297"), crate = "crate::vendor::near_sdk")]
pub enum Nep297Event {
    #[event_version("1.0.0")]
    Incremented { value: u64 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn works_without_a_direct_near_sdk_dependency() {
        let mut counter = Counter::new();
        assert_eq!(counter.increment(), 1);
        assert_eq!(counter.increment(), 2);
        assert_eq!(counter.get(), 2);
    }

    #[test]
    fn nep297_event_emits_without_a_direct_near_sdk_dependency() {
        let event = Nep297Event::Incremented { value: 1 };
        event.emit();

        let logs = vendor::near_sdk::test_utils::get_logs();
        assert_eq!(logs.len(), 1);
        assert!(logs[0].starts_with("EVENT_JSON:"));
        assert!(logs[0].contains(r#""standard":"nep297""#));
        assert!(logs[0].contains(r#""version":"1.0.0""#));
    }
}
