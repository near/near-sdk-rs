use std::borrow::Borrow;

use borsh::{BorshDeserialize, BorshSerialize};

use super::{UnorderedMap, ERR_NOT_EXIST};
use crate::env;

impl<K, V> Extend<(K, V)> for UnorderedMap<K, V>
where
    K: BorshSerialize + Ord + BorshDeserialize + Clone,
    V: BorshSerialize + BorshDeserialize,
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

impl<K, V, Q: ?Sized> core::ops::Index<&Q> for UnorderedMap<K, V>
where
    K: BorshSerialize + Ord + Clone + Borrow<Q>,
    V: BorshSerialize + BorshDeserialize,
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
