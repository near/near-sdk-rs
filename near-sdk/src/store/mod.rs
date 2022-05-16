//! Collections and types used when interacting with storage.
//!
//! This module is the updated version of [`near_sdk::collections`](crate::collections) where the
//! data structures are more optimized and have a closer API to [`std::collections`].
//!
//! These collections are more scalable versions of [`std::collections`] when used as contract
//! state because it allows values to be lazily loaded and stored based on what is actually
//! interacted with.
//!
//! The collections are as follows:
//!
//! Sequences:
//!
//! - [`Vector`]: Analygous to [`Vec`] but not contiguous and persisted to storage.
//!
//! Maps:
//!
//! - [`LookupMap`] (`unstable`): Wrapper around key-value storage interactions, similar to
//! [`UnorderedMap`]/[`std::collections::HashMap`] except that keys are not persisted and cannot be
//! iterated over.
//!
//! - [`UnorderedMap`] (`unstable`): Storage version of [`std::collections::HashMap`]. No ordering
//! guarantees.
//!
//! - [`TreeMap`] (`unstable`): Storage version of [`std::collections::BTreeMap`]. Ordered by key,
//! which comes at the cost of more expensive lookups and iteration.
//!
//! Sets:
//!
//! - [`LookupSet`] (`unstable`): Non-iterable storage version of [`std::collections::HashSet`].
//!
//! - [`UnorderedSet`] (`unstable`): Analygous to [`std::collections::HashSet`], and is an iterable
//! version of [`LookupSet`] and persisted to storage.
//!
//! Basic Types:
//!
//! - [`Lazy<T>`](Lazy) (`unstable`): Lazily loaded type that can be used in place of a type `T`.
//! Will only be loaded when interacted with and will persist on [`Drop`].
//!
//! - [`LazyOption<T>`](LazyOption) (`unstable`): Lazily loaded, optional type that can be used in
//! place of a type [`Option<T>`](Option). Will only be loaded when interacted with and will
//! persist on [`Drop`].

#[cfg(feature = "unstable")]
mod lazy;
#[cfg(feature = "unstable")]
pub use lazy::Lazy;

#[cfg(feature = "unstable")]
mod lazy_option;
#[cfg(feature = "unstable")]
pub use lazy_option::LazyOption;

pub mod vec;
pub use vec::Vector;

#[cfg(feature = "unstable")]
pub mod lookup_map;
#[cfg(feature = "unstable")]
pub use self::lookup_map::LookupMap;

#[cfg(feature = "unstable")]
mod lookup_set;
#[cfg(feature = "unstable")]
pub use self::lookup_set::LookupSet;

#[cfg(feature = "unstable")]
pub mod unordered_map;
#[cfg(feature = "unstable")]
pub use self::unordered_map::UnorderedMap;

#[cfg(feature = "unstable")]
pub mod unordered_set;
#[cfg(feature = "unstable")]
pub use self::unordered_set::UnorderedSet;

#[cfg(feature = "unstable")]
pub mod tree_map;
#[cfg(feature = "unstable")]
pub use self::tree_map::TreeMap;

mod index_map;
pub(crate) use self::index_map::IndexMap;

#[cfg(feature = "unstable")]
pub(crate) mod free_list;
#[cfg(feature = "unstable")]
pub(crate) use self::free_list::FreeList;

/// Storage key hash function types and trait to override map hash functions.
#[cfg(feature = "unstable")]
pub mod key;

pub(crate) const ERR_INCONSISTENT_STATE: &str =
    "The collection is an inconsistent state. Did previous smart \
        contract execution terminate unexpectedly?";

#[cfg(feature = "unstable")]
pub(crate) const ERR_NOT_EXIST: &str = "Key does not exist in map";
