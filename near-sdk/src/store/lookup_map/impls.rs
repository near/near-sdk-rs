use borsh::{BorshDeserialize, BorshSerialize};

use super::{LookupMap, ERR_INDEX_OUT_OF_BOUNDS};
use crate::{env, hash::CryptoHash};

impl<K, V, H> Drop for LookupMap<K, V, H>
where
    K: BorshSerialize + Ord,
    V: BorshSerialize,
    H: CryptoHash<Digest = [u8; 32]>,
{
    fn drop(&mut self) {
        self.flush()
    }
}

impl<K, V, H> Extend<(K, V)> for LookupMap<K, V, H>
where
    K: BorshSerialize + Ord,
    V: BorshSerialize,
    H: CryptoHash<Digest = [u8; 32]>,
{
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = (K, V)>,
    {
        for (key, value) in iter {
            self.set(key, Some(value))
        }
    }
}

impl<K, V, H> core::ops::Index<K> for LookupMap<K, V, H>
where
    K: BorshSerialize + Ord + Clone,
    V: BorshSerialize + BorshDeserialize,
    H: CryptoHash<Digest = [u8; 32]>,
{
    type Output = V;

    fn index(&self, index: K) -> &Self::Output {
        self.get(&index).unwrap_or_else(|| env::panic(ERR_INDEX_OUT_OF_BOUNDS))
    }
}

impl<K, V, H> core::ops::IndexMut<K> for LookupMap<K, V, H>
where
    K: BorshSerialize + Ord + Clone,
    V: BorshSerialize + BorshDeserialize,
    H: CryptoHash<Digest = [u8; 32]>,
{
    fn index_mut(&mut self, index: K) -> &mut Self::Output {
        self.get_mut(&index).unwrap_or_else(|| env::panic(ERR_INDEX_OUT_OF_BOUNDS))
    }
}
