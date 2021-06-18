use borsh::{BorshDeserialize, BorshSerialize};

use super::LazyOption;

impl<T> Drop for LazyOption<T>
where
    T: BorshSerialize,
{
    fn drop(&mut self) {
        self.flush()
    }
}

impl<T> core::ops::Deref for LazyOption<T>
where
    T: BorshSerialize + BorshDeserialize,
{
    type Target = Option<T>;

    fn deref(&self) -> &Self::Target {
        Self::get(self)
    }
}

impl<T> core::ops::DerefMut for LazyOption<T>
where
    T: BorshSerialize + BorshDeserialize,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        Self::get_mut(self)
    }
}
