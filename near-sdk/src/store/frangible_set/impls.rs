use super::FrangibleUnorderedSet;
use crate::store::key::ToKey;
use borsh::{BorshDeserialize, BorshSerialize};

impl<T, H> Extend<T> for FrangibleUnorderedSet<T, H>
where
    T: BorshSerialize + Ord + BorshDeserialize + Clone,
    H: ToKey,
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
