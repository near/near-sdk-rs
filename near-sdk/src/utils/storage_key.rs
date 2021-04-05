use borsh::BorshSerialize;

/// Converts Self into a `Vec<u8>` that is used for a storage key.
pub trait IntoStorageKey {
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

/// Converts a Borsh serializable object into a `Vec<u8>` that is used for a storage key.
///
/// ```
/// use near_sdk::borsh::BorshSerialize;
/// use near_sdk::BorshStorageKey;
///
/// #[derive(BorshSerialize)]
/// enum StorageKey {
///     FungibleToken,
///     Metadata,
/// }
///
/// impl BorshStorageKey for StorageKey {}
/// ```
pub trait BorshStorageKey: BorshSerialize {}

impl<T> IntoStorageKey for T
where
    T: BorshStorageKey,
{
    fn into_storage_key(self) -> Vec<u8> {
        self.try_to_vec().unwrap()
    }
}
