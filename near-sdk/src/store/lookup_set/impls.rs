use super::LookupSet;
use crate::crypto_hash::CryptoHasher;
use borsh::BorshSerialize;

impl<T, H> Extend<T> for LookupSet<T, H>
where
    T: BorshSerialize + Ord,
    H: CryptoHasher<Digest = [u8; 32]>,
{
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        self.map.extend(iter.into_iter().map(|k| (k, ())))
    }
}
