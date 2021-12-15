use super::LookupSet;
use borsh::BorshSerialize;

impl<T> Extend<T> for LookupSet<T>
where
    T: BorshSerialize + Ord,
{
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        self.map.extend(iter.into_iter().map(|k| (k, ())))
    }
}
