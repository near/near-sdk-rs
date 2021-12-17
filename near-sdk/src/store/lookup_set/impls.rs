use super::LookupSet;
use crate::store::ToKey;
use borsh::BorshSerialize;

impl<T, H> Extend<T> for LookupSet<T, H>
where
    T: BorshSerialize + Ord,
    H: ToKey,
    <H as ToKey>::KeyType: AsRef<[u8]>,
{
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        self.map.extend(iter.into_iter().map(|k| (k, ())))
    }
}
