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
/// use near_sdk::collections::LookupMap;
///
/// #[derive(BorshSerialize)]
///  enum StorageKey {
///     FungibleToken,
///     Metadata { sub_key: String },
/// }
///
/// impl BorshStorageKey for StorageKey {}
///
/// let lookup_map_1: LookupMap<u64, String> = LookupMap::new(StorageKey::Metadata { sub_key: String::from("yo") });
/// let lookup_map_2: LookupMap<String, String> = LookupMap::new(StorageKey::FungibleToken);
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

/// Helper macro to create a new enum `StorageKey`. This enum implements `BorshStorageKey`, so
/// it can be passed to the persistent collections to create a unique prefix for storage.
///
/// ```
/// use near_sdk::storage_key;
/// use near_sdk::collections::LookupMap;
///
/// storage_key! {
///     FungibleToken,
///     Metadata { sub_key: String },
/// }
///
/// let lookup_map_1: LookupMap<u64, String> = LookupMap::new(StorageKey::Metadata { sub_key: String::from("yo") });
/// let lookup_map_2: LookupMap<String, String> = LookupMap::new(StorageKey::FungibleToken);
/// ```
///
/// ```
/// use near_sdk::storage_key;
///
/// storage_key! {
///     Accounts,
/// }
/// ```
#[macro_export]
macro_rules! storage_key {
    {$($arg:tt)*} => {
        #[derive($crate::borsh::BorshSerialize)]
        pub(crate) enum StorageKey {
            $($arg)*
        }
        impl $crate::BorshStorageKey for StorageKey {}
    };
}
