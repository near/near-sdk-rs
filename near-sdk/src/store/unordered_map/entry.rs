use borsh::{BorshDeserialize, BorshSerialize};

use super::ValueAndIndex;
use crate::store::{lookup_map as lm, FreeList};

/// A view into a single entry in the map, which can be vacant or occupied.
pub enum Entry<'a, K: 'a, V: 'a>
where
    K: BorshSerialize,
{
    Occupied(OccupiedEntry<'a, K, V>),
    Vacant(VacantEntry<'a, K, V>),
}

impl<'a, K, V> Entry<'a, K, V>
where
    K: BorshSerialize,
{
    pub(super) fn new(
        lm_entry: lm::Entry<'a, K, ValueAndIndex<V>>,
        keys: &'a mut FreeList<K>,
    ) -> Self {
        match lm_entry {
            lm::Entry::Occupied(value_entry) => Self::Occupied(OccupiedEntry { value_entry, keys }),
            lm::Entry::Vacant(value_entry) => Self::Vacant(VacantEntry { value_entry, keys }),
        }
    }
}

impl<'a, K, V> Entry<'a, K, V>
where
    K: BorshSerialize,
{
    /// Returns a reference to this entry's key.
    ///
    /// # Examples
    ///
    /// ```
    /// use near_sdk::store::UnorderedMap;
    ///
    /// let mut map: UnorderedMap<String, u32> = UnorderedMap::new(b"m");
    /// assert_eq!(map.entry("poneyland".to_string()).key(), "poneyland");
    /// ```
    pub fn key(&self) -> &K {
        match self {
            Entry::Occupied(entry) => entry.key(),
            Entry::Vacant(entry) => entry.key(),
        }
    }
}

impl<'a, K, V> Entry<'a, K, V>
where
    K: BorshSerialize + BorshDeserialize + Clone,
{
    /// Ensures a value is in the entry by inserting the default if empty, and returns
    /// a mutable reference to the value in the entry.
    ///
    /// # Examples
    ///
    /// ```
    /// use near_sdk::store::UnorderedMap;
    ///
    /// let mut map: UnorderedMap<String, u32> = UnorderedMap::new(b"m");
    ///
    /// map.entry("poneyland".to_string()).or_insert(3);
    /// assert_eq!(map["poneyland"], 3);
    ///
    /// *map.entry("poneyland".to_string()).or_insert(10) *= 2;
    /// assert_eq!(map["poneyland"], 6);
    /// ```
    pub fn or_insert(self, default: V) -> &'a mut V {
        self.or_insert_with(|| default)
    }

    /// Ensures a value is in the entry by inserting the result of the default function if empty,
    /// and returns a mutable reference to the value in the entry.
    ///
    /// # Examples
    ///
    /// ```
    /// use near_sdk::store::UnorderedMap;
    ///
    /// let mut map: UnorderedMap<String, String> = UnorderedMap::new(b"m");
    /// let s = "hoho".to_string();
    ///
    /// map.entry("poneyland".to_string()).or_insert_with(|| s);
    ///
    /// assert_eq!(map["poneyland"], "hoho".to_string());
    /// ```
    pub fn or_insert_with<F: FnOnce() -> V>(self, default: F) -> &'a mut V {
        self.or_insert_with_key(|_| default())
    }

    /// Ensures a value is in the entry by inserting, if empty, the result of the default function.
    /// This method allows for generating key-derived values for insertion by providing the default
    /// function a reference to the key that was moved during the `.entry(key)` method call.
    ///
    /// The reference to the moved key is provided so that cloning or copying the key is
    /// unnecessary, unlike with `.or_insert_with(|| ... )`.
    ///
    /// # Examples
    ///
    /// ```
    /// use near_sdk::store::UnorderedMap;
    ///
    /// let mut map: UnorderedMap<String, u32> = UnorderedMap::new(b"m");
    ///
    /// map.entry("poneyland".to_string()).or_insert_with_key(|key| key.chars().count() as u32);
    ///
    /// assert_eq!(map["poneyland"], 9);
    /// ```
    pub fn or_insert_with_key<F: FnOnce(&K) -> V>(self, default: F) -> &'a mut V
    where
        K: BorshDeserialize,
    {
        match self {
            Self::Occupied(entry) => entry.into_mut(),
            Self::Vacant(entry) => {
                let value = default(entry.key());
                entry.insert(value)
            }
        }
    }

    /// Ensures a value is in the entry by inserting the default value if empty,
    /// and returns a mutable reference to the value in the entry.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() {
    /// use near_sdk::store::UnorderedMap;
    ///
    /// let mut map: UnorderedMap<String, Option<u32>> = UnorderedMap::new(b"m");
    /// map.entry("poneyland".to_string()).or_default();
    ///
    /// assert_eq!(map["poneyland"], None);
    /// # }
    /// ```
    pub fn or_default(self) -> &'a mut V
    where
        V: Default,
    {
        match self {
            Self::Occupied(entry) => entry.into_mut(),
            Self::Vacant(entry) => entry.insert(Default::default()),
        }
    }

    /// Provides in-place mutable access to an occupied entry before any
    /// potential inserts into the map.
    ///
    /// # Examples
    ///
    /// ```
    /// use near_sdk::store::UnorderedMap;
    ///
    /// let mut map: UnorderedMap<String, u32> = UnorderedMap::new(b"m");
    ///
    /// map.entry("poneyland".to_string())
    ///    .and_modify(|e| { *e += 1 })
    ///    .or_insert(42);
    /// assert_eq!(map["poneyland"], 42);
    ///
    /// map.entry("poneyland".to_string())
    ///    .and_modify(|e| { *e += 1 })
    ///    .or_insert(42);
    /// assert_eq!(map["poneyland"], 43);
    /// ```
    pub fn and_modify<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut V),
    {
        if let Self::Occupied(entry) = &mut self {
            f(entry.get_mut());
        }
        self
    }
}

/// View into an occupied entry in a [`UnorderedMap`](super::UnorderedMap).
/// This is part of the [`Entry`] enum.
pub struct OccupiedEntry<'a, K, V>
where
    K: BorshSerialize,
{
    value_entry: lm::OccupiedEntry<'a, K, ValueAndIndex<V>>,
    keys: &'a mut FreeList<K>,
}

impl<'a, K, V> OccupiedEntry<'a, K, V>
where
    K: BorshSerialize,
{
    /// Gets a reference to the key in the entry.
    pub fn key(&self) -> &K {
        self.value_entry.key()
    }

    /// Take the ownership of the key and value from the map.
    ///
    /// # Examples
    ///
    /// ```
    /// use near_sdk::store::UnorderedMap;
    /// use near_sdk::store::unordered_map::Entry;
    ///
    /// let mut map: UnorderedMap<String, u32> = UnorderedMap::new(b"m");
    /// map.entry("poneyland".to_string()).or_insert(12);
    ///
    /// if let Entry::Occupied(o) = map.entry("poneyland".to_string()) {
    ///     // We delete the entry from the map.
    ///     o.remove_entry();
    /// }
    ///
    /// assert_eq!(map.contains_key("poneyland"), false);
    /// ```
    pub fn remove_entry(self) -> (K, V)
    where
        K: BorshDeserialize,
    {
        let (key, value) = self.value_entry.remove_entry();
        self.keys.remove(value.key_index);
        (key, value.value)
    }

    /// Gets a reference to the value in the entry.
    ///
    /// # Examples
    ///
    /// ```
    /// use near_sdk::store::UnorderedMap;
    /// use near_sdk::store::unordered_map::Entry;
    ///
    /// let mut map: UnorderedMap<String, u32> = UnorderedMap::new(b"m");
    /// map.entry("poneyland".to_string()).or_insert(12);
    ///
    /// if let Entry::Occupied(o) = map.entry("poneyland".to_string()) {
    ///     assert_eq!(o.get(), &12);
    /// }
    /// ```
    pub fn get(&self) -> &V {
        &self.value_entry.get().value
    }

    /// Gets a mutable reference to the value in the entry.
    ///
    /// If you need a reference to the `OccupiedEntry` which may outlive the
    /// destruction of the `Entry` value, see [`into_mut`].
    ///
    /// [`into_mut`]: Self::into_mut
    ///
    /// # Examples
    ///
    /// ```
    /// use near_sdk::store::UnorderedMap;
    /// use near_sdk::store::unordered_map::Entry;
    ///
    /// let mut map: UnorderedMap<String, u32> = UnorderedMap::new(b"m");
    /// map.entry("poneyland".to_string()).or_insert(12);
    ///
    /// assert_eq!(map["poneyland"], 12);
    /// if let Entry::Occupied(mut o) = map.entry("poneyland".to_string()) {
    ///     *o.get_mut() += 10;
    ///     assert_eq!(*o.get(), 22);
    ///
    ///     // We can use the same Entry multiple times.
    ///     *o.get_mut() += 2;
    /// }
    ///
    /// assert_eq!(map["poneyland"], 24);
    /// ```
    pub fn get_mut(&mut self) -> &mut V {
        &mut self.value_entry.get_mut().value
    }

    /// Converts the `OccupiedEntry` into a mutable reference to the value in the entry
    /// with a lifetime bound to the map itself.
    ///
    /// If you need multiple references to the `OccupiedEntry`, see [`get_mut`].
    ///
    /// [`get_mut`]: Self::get_mut
    ///
    /// # Examples
    ///
    /// ```
    /// use near_sdk::store::UnorderedMap;
    /// use near_sdk::store::unordered_map::Entry;
    ///
    /// let mut map: UnorderedMap<String, u32> = UnorderedMap::new(b"m");
    /// map.entry("poneyland".to_string()).or_insert(12);
    ///
    /// assert_eq!(map["poneyland"], 12);
    /// if let Entry::Occupied(o) = map.entry("poneyland".to_string()) {
    ///     *o.into_mut() += 10;
    /// }
    ///
    /// assert_eq!(map["poneyland"], 22);
    /// ```
    pub fn into_mut(self) -> &'a mut V {
        &mut self.value_entry.into_mut().value
    }

    /// Sets the value of the entry, and returns the entry's old value.
    ///
    /// # Examples
    ///
    /// ```
    /// use near_sdk::store::UnorderedMap;
    /// use near_sdk::store::unordered_map::Entry;
    ///
    /// let mut map: UnorderedMap<String, u32> = UnorderedMap::new(b"m");
    /// map.entry("poneyland".to_string()).or_insert(12);
    ///
    /// if let Entry::Occupied(mut o) = map.entry("poneyland".to_string()) {
    ///     assert_eq!(o.insert(15), 12);
    /// }
    ///
    /// assert_eq!(map["poneyland"], 15);
    /// ```
    pub fn insert(&mut self, value: V) -> V {
        core::mem::replace(&mut self.value_entry.get_mut().value, value)
    }

    /// Takes the value out of the entry, and returns it.
    ///
    /// # Examples
    ///
    /// ```
    /// use near_sdk::store::UnorderedMap;
    /// use near_sdk::store::unordered_map::Entry;
    ///
    /// let mut map: UnorderedMap<String, u32> = UnorderedMap::new(b"m");
    /// map.entry("poneyland".to_string()).or_insert(12);
    ///
    /// if let Entry::Occupied(o) = map.entry("poneyland".to_string()) {
    ///     assert_eq!(o.remove(), 12);
    /// }
    ///
    /// assert_eq!(map.contains_key("poneyland"), false);
    /// ```
    pub fn remove(self) -> V
    where
        K: BorshDeserialize,
    {
        self.remove_entry().1
    }
}

/// View into a vacant entry in a [`UnorderedMap`](super::UnorderedMap).
/// This is part of the [`Entry`] enum.
pub struct VacantEntry<'a, K, V>
where
    K: BorshSerialize,
{
    value_entry: lm::VacantEntry<'a, K, ValueAndIndex<V>>,
    keys: &'a mut FreeList<K>,
}

impl<'a, K, V> VacantEntry<'a, K, V>
where
    K: BorshSerialize,
{
    /// Gets a reference to the key that would be used when inserting a value
    /// through the `VacantEntry`.
    pub fn key(&self) -> &K {
        self.value_entry.key()
    }

    /// Take ownership of the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use near_sdk::store::UnorderedMap;
    /// use near_sdk::store::unordered_map::Entry;
    ///
    /// let mut map: UnorderedMap<String, u32> = UnorderedMap::new(b"m");
    ///
    /// if let Entry::Vacant(v) = map.entry("poneyland".to_string()) {
    ///     v.into_key();
    /// }
    /// ```
    pub fn into_key(self) -> K {
        self.value_entry.into_key()
    }

    /// Sets the value of the entry with the `VacantEntry`'s key,
    /// and returns a mutable reference to it.
    ///
    /// # Examples
    ///
    /// ```
    /// use near_sdk::store::UnorderedMap;
    /// use near_sdk::store::unordered_map::Entry;
    ///
    /// let mut map: UnorderedMap<String, u32> = UnorderedMap::new(b"m");
    ///
    /// if let Entry::Vacant(o) = map.entry("poneyland".to_string()) {
    ///     o.insert(37);
    /// }
    /// assert_eq!(map["poneyland"], 37);
    /// ```
    pub fn insert(self, value: V) -> &'a mut V
    where
        K: BorshDeserialize + Clone,
    {
        // Vacant entry so we know key doesn't exist
        let key_index = self.keys.insert(self.key().to_owned());
        &mut self.value_entry.insert(ValueAndIndex { value, key_index }).value
    }
}
