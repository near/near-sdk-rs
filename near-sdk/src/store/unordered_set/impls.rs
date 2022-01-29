use super::UnorderedSet;
use crate::crypto_hash::CryptoHasher;
use borsh::{BorshDeserialize, BorshSerialize};

impl<T, H> Extend<T> for UnorderedSet<T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize + Clone,
    H: CryptoHasher<Digest = [u8; 32]>,
{
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        for value in iter {
            self.insert(value);
        }
    }
}
