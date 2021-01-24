use near_sdk::json_types::{U64, U128};

/// Raw type for duration in nanoseconds
pub type Duration = u64;

/// Duration in nanosecond wrapped into a struct for JSON serialization as a string.
pub type WrappedDuration = U64;

/// Raw type for timestamp in nanoseconds
pub type Timestamp = u64;

/// Timestamp in nanosecond wrapped into a struct for JSON serialization as a string.
pub type WrappedTimestamp = U64;

/// Balance wrapped into a struct for JSON serialization as a string.
pub type WrappedBalance = U128;
