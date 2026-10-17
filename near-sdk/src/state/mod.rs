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
        Self: StateRead,
    {
        <Self as StateRead>::read_state(Self::state_key())
    }

    #[inline]
    #[track_caller]
    fn state_write(&self) -> bool
    where
        Self: StateWrite,
    {
        <Self as StateWrite>::write_state(self, Self::state_key())
    }
}

/// How a contract root is loaded by the `#[near]` wrapper.
///
/// Every Borsh type gets the v1 behaviour (one blob under `state_key()`) through the blanket
/// impl. A storage v2 root (`contract_state(layout = v2)`) is not Borsh and gets a generated
/// impl instead. This keeps the wrapper's generated code identical for v1 and v2 contracts.
pub trait StateRead: Sized {
    fn read_state(key: &[u8]) -> Option<Self>;
}

/// How a contract root is stored by the `#[near]` wrapper. See [`StateRead`].
pub trait StateWrite {
    fn write_state(&self, key: &[u8]) -> bool;
}

impl<T: BorshDeserialize> StateRead for T {
    #[inline]
    #[track_caller]
    fn read_state(key: &[u8]) -> Option<Self> {
        env::storage_read(key).map(|data| {
            borsh::from_slice(&data)
                .unwrap_or_else(|_| env::panic_str("Cannot deserialize the contract state."))
        })
    }
}

impl<T: BorshSerialize> StateWrite for T {
    #[inline]
    #[track_caller]
    fn write_state(&self, key: &[u8]) -> bool {
        env::storage_write(
            key,
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
    At, Group, GroupMut, GroupedMap, IterEntry, IterMap, IterSet, KeyPage, Lazy, Map, MapEntry,
    Page, Set, Vec,
};
#[cfg(feature = "unstable")]
pub use key::{Path, StorageKey};
#[cfg(feature = "unstable")]
pub use record::RecordError;
#[cfg(feature = "unstable")]
pub use root::{Root, __private, flush_root, load_root, raw_root, root_exists};
#[cfg(feature = "unstable")]
pub use value::{FieldDefault, Record, Removable, ScalarNotInView, Value, ViewField};
