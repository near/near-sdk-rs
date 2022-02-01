use std::borrow::Borrow;

use borsh::{BorshDeserialize, BorshSerialize};

use super::{ToKey, UnorderedMap, ERR_NOT_EXIST};
use crate::env;

impl<K, V, H> Extend<(K, V)> for UnorderedMap<K, V, H>
where
    K: BorshSerialize + Ord + BorshDeserialize + Clone,
    V: BorshSerialize + BorshDeserialize,
    H: ToKey,
{
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = (K, V)>,
    {
        for (key, value) in iter {
            self.insert(key, value);
        }
    }
}

impl<K, V, H, Q: ?Sized> core::ops::Index<&Q> for UnorderedMap<K, V, H>
where
    K: BorshSerialize + Ord + Clone + Borrow<Q>,
    V: BorshSerialize + BorshDeserialize,
    H: ToKey,

    Q: BorshSerialize + ToOwned<Owned = K>,
{
    type Output = V;

    /// Returns reference to value corresponding to key.
    ///
    /// # Panics
    ///
    /// Panics if the key does not exist in the map
    fn index(&self, index: &Q) -> &Self::Output {
        self.get(index).unwrap_or_else(|| env::panic_str(ERR_NOT_EXIST))
    }
}
