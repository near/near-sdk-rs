use borsh::BorshSerialize;

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

/// Converts a Borsh serializable object into a `Vec<u8>` that is used for a storage key.
///
/// ```
/// use near_sdk::borsh::BorshSerialize;
/// use near_sdk::BorshIntoStorageKey;
/// use near_sdk::collections::LookupMap;
///
/// #[derive(BorshSerialize)]
///  enum StorageKey {
///     FungibleToken,
///     Metadata { sub_key: String },
/// }
///
/// impl BorshIntoStorageKey for StorageKey {}
///
/// let lookup_map_1: LookupMap<u64, String> = LookupMap::new(StorageKey::Metadata { sub_key: String::from("yo") });
/// let lookup_map_2: LookupMap<String, String> = LookupMap::new(StorageKey::FungibleToken);
/// ```
pub trait BorshIntoStorageKey: BorshSerialize {}

impl<T> IntoStorageKey for T
where
    T: BorshIntoStorageKey,
{
    fn into_storage_key(self) -> Vec<u8> {
        self.try_to_vec().unwrap()
    }
}
