use borsh::{BorshDeserialize, BorshSerialize};

use crate::{env, errors};

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
            borsh::from_slice(&data).unwrap_or_else(|_| {
                env::panic_err(errors::BorshDeserializeError::new("contract state"))
            })
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
            &borsh::to_vec(self).unwrap_or_else(|_| {
                env::panic_err(errors::BorshSerializeError::new("contract state"))
            }),
        )
    }
}
