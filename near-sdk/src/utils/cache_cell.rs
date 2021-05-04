use core::{cell::UnsafeCell, fmt, fmt::Debug, ptr::NonNull};

pub struct CacheCell<T: ?Sized> {
    inner: UnsafeCell<T>,
}

impl<T> CacheCell<T> {
    pub fn new(value: T) -> Self {
        Self { inner: UnsafeCell::new(value) }
    }

    #[allow(dead_code)]
    pub fn into_inner(self) -> T {
        self.inner.into_inner()
    }
}

impl<T> Debug for CacheCell<T>
where
    T: ?Sized + Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        <T as Debug>::fmt(self.as_inner(), f)
    }
}

impl<T> From<T> for CacheCell<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T> Default for CacheCell<T>
where
    T: Default,
{
    fn default() -> Self {
        Self::new(<T as Default>::default())
    }
}

impl<T> CacheCell<T>
where
    T: ?Sized,
{
    pub fn as_inner(&self) -> &T {
        // TODO doc safety
        unsafe { &*self.inner.get() }
    }

    pub fn as_inner_mut(&mut self) -> &mut T {
        // TODO doc safety
        unsafe { &mut *self.inner.get() }
    }

    pub fn get_ptr(&self) -> NonNull<T> {
        // TODO doc safety
        unsafe { NonNull::new_unchecked(self.inner.get()) }
    }
}
