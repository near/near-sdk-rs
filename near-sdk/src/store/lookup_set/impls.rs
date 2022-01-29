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
        iter.into_iter().for_each(move |elem| {
            self.put(elem);
        });
    }
}
