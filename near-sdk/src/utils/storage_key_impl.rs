/// Converts Self into a [`Vec<u8>`] that is used for a storage key through [`into_storage_key`].
///
/// [`into_storage_key`]: IntoStorageKey::into_storage_key
pub trait IntoStorageKey {
    /// Consumes self and returns [`Vec<u8>`] bytes which are used as a storage key.
    fn into_storage_key(self) -> Vec<u8>;
}

impl IntoStorageKey for Vec<u8> {
    #[inline]
    fn into_storage_key(self) -> Vec<u8> {
        self
    }
}

impl<'a> IntoStorageKey for &'a [u8] {
    #[inline]
    fn into_storage_key(self) -> Vec<u8> {
        self.to_vec()
    }
}

impl<'a> IntoStorageKey for &'a [u8; 1] {
    #[inline]
    fn into_storage_key(self) -> Vec<u8> {
        self.to_vec()
    }
}

impl IntoStorageKey for u8 {
    #[inline]
    fn into_storage_key(self) -> Vec<u8> {
        vec![self]
    }
}
