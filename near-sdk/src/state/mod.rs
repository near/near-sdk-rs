use borsh::{BorshDeserialize, BorshSerialize};

use crate::env;

pub trait ContractState {
    #[inline]
    fn state_key() -> &'static [u8] {
        b"STATE"
    }

    #[inline]
    fn state_exists() -> bool {
        env::storage_has_key(Self::state_key())
    }

    #[inline]
    #[track_caller]
    fn state_read() -> Option<Self>
    where
        Self: BorshDeserialize,
    {
        env::storage_read(Self::state_key()).map(|data| {
            borsh::from_slice(&data)
                .unwrap_or_else(|_| env::panic_str("Cannot deserialize the contract state."))
        })
    }

    #[inline]
    #[track_caller]
    fn state_write(&self) -> bool
    where
        Self: BorshSerialize,
    {
        env::storage_write(
            Self::state_key(),
            &borsh::to_vec(self)
                .unwrap_or_else(|_| env::panic_str("Cannot serialize the contract state.")),
        )
    }
}

// ---- storage v2 prototype (see PROTOTYPE_NOTES.md) ----

#[cfg(feature = "unstable")]
mod cache;
#[cfg(feature = "unstable")]
mod collections;
#[cfg(feature = "unstable")]
pub mod key;
#[cfg(feature = "unstable")]
pub mod record;
#[cfg(feature = "unstable")]
#[doc(hidden)]
pub mod testing;
#[cfg(feature = "unstable")]
mod value;
#[cfg(feature = "unstable")]
mod root;
#[cfg(feature = "unstable")]
#[cfg(test)]
mod tests;

#[cfg(feature = "unstable")]
pub use collections::{
    Group, GroupMut, GroupedMap, IterEntry, IterMap, IterSet, Lazy, Map, MapEntry, Page, Set, Vec,
};
#[cfg(feature = "unstable")]
pub use key::{Path, StorageKey};
#[cfg(feature = "unstable")]
pub use record::RecordError;
#[cfg(feature = "unstable")]
pub use root::{Root, __private, flush_root, load_root, raw_root};
#[cfg(feature = "unstable")]
pub use value::{Record, Removable, Value, removable_leaf};
