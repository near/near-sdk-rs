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

impl<T> std::fmt::Debug for LazyOption<T>
where
    T: std::fmt::Debug + BorshSerialize + BorshDeserialize,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if cfg!(feature = "expensive-debug") {
            self.get().fmt(f)
        } else {
            f.debug_struct("LazyOption")
                .field("storage_key", &self.prefix)
                .field("cache", &self.cache.get())
                .finish()
        }
    }
}
