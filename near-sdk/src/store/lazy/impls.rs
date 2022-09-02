use borsh::{BorshDeserialize, BorshSerialize};

use super::Lazy;

impl<T> Drop for Lazy<T>
where
    T: BorshSerialize,
{
    fn drop(&mut self) {
        self.flush()
    }
}

impl<T> core::ops::Deref for Lazy<T>
where
    T: BorshSerialize + BorshDeserialize,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        Self::get(self)
    }
}

impl<T> core::ops::DerefMut for Lazy<T>
where
    T: BorshSerialize + BorshDeserialize,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        Self::get_mut(self)
    }
}

impl<T> core::cmp::PartialEq for Lazy<T>
where
    T: PartialEq + BorshSerialize + BorshDeserialize,
{
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(self.get(), other.get())
    }
}

impl<T> core::cmp::Eq for Lazy<T> where T: Eq + BorshSerialize + BorshDeserialize {}

impl<T> core::cmp::PartialOrd for Lazy<T>
where
    T: PartialOrd + BorshSerialize + BorshDeserialize,
{
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        PartialOrd::partial_cmp(self.get(), other.get())
    }
    fn lt(&self, other: &Self) -> bool {
        PartialOrd::lt(self.get(), other.get())
    }
    fn le(&self, other: &Self) -> bool {
        PartialOrd::le(self.get(), other.get())
    }
    fn ge(&self, other: &Self) -> bool {
        PartialOrd::ge(self.get(), other.get())
    }
    fn gt(&self, other: &Self) -> bool {
        PartialOrd::gt(self.get(), other.get())
    }
}

impl<T> core::cmp::Ord for Lazy<T>
where
    T: core::cmp::Ord + BorshSerialize + BorshDeserialize,
{
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        Ord::cmp(self.get(), other.get())
    }
}

impl<T> core::convert::AsRef<T> for Lazy<T>
where
    T: BorshSerialize + BorshDeserialize,
{
    fn as_ref(&self) -> &T {
        Self::get(self)
    }
}

impl<T> core::convert::AsMut<T> for Lazy<T>
where
    T: BorshSerialize + BorshDeserialize,
{
    fn as_mut(&mut self) -> &mut T {
        Self::get_mut(self)
    }
}

impl<T> std::fmt::Debug for Lazy<T>
where
    T: std::fmt::Debug + BorshSerialize + BorshDeserialize,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if cfg!(feature = "expensive-debug") {
            self.get().fmt(f)
        } else {
            f.debug_struct("Lazy")
                .field("storage_key", &self.storage_key)
                .field("cache", &self.cache.get())
                .finish()
        }
    }
}
