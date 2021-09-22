use borsh::{BorshDeserialize, BorshSerialize};

use super::iter::{Iter, IterMut};
use super::Bucket;

impl<'a, T> IntoIterator for &'a Bucket<T>
where
    T: BorshSerialize + BorshDeserialize,
{
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut Bucket<T>
where
    T: BorshSerialize + BorshDeserialize,
{
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<T> Extend<T> for Bucket<T>
where
    T: BorshSerialize + BorshDeserialize,
{
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        for item in iter {
            self.insert(item);
        }
    }
}
