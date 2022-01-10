use super::LookupSet;
use crate::store::key::ToKey;
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
        iter.into_iter().for_each(move |elem| {
            self.put(elem);
        });
    }
}
