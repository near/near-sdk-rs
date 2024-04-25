use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::BTreeMap;

/// This map is used as an append-only cache of keys to values. Insertions in the map can be done
/// with only an immutable reference because the values are boxed and any insertion will not create
/// dangling pointers for existing references.
pub(crate) struct StableMap<K, V> {
    map: RefCell<BTreeMap<K, Box<V>>>,
}

impl<K: Ord, V> Default for StableMap<K, V> {
    fn default() -> Self {
        Self { map: Default::default() }
    }
}

impl<K, V> StableMap<K, V> {
    /// Gets reference to value if it exists in the map. If it does not exist, the default value
    /// will be used to initialize before returning a reference to it.
    pub(crate) fn get(&self, k: K) -> &V
    where
        K: Ord,
        V: Default,
    {
        let mut map = self.map.borrow_mut();
        let v: &mut Box<V> = map.entry(k).or_default();
        let v: &V = &*v;
        // SAFETY: here, we extend the lifetime of `V` from local `RefCell`
        // borrow to the `&self`. This is valid because we only append to the
        // map via `&` reference, and the values are boxed, so we have stability
        // of addresses.
        unsafe { &*(v as *const V) }
    }
    /// Gets mutable reference to value if it exists in the map. If it does not exist, the default
    /// value will be used to initialize before returning a reference to it.
    pub(crate) fn get_mut(&mut self, k: K) -> &mut V
    where
        K: Ord,
        V: Default,
    {
        &mut *self.map.get_mut().entry(k).or_default()
    }
    pub(crate) fn inner(&mut self) -> &mut BTreeMap<K, Box<V>> {
        self.map.get_mut()
    }
    pub(crate) fn map_value_ref<Q: ?Sized, F, T>(&self, k: &Q, f: F) -> Option<T>
    where
        K: Borrow<Q> + Ord,
        Q: Ord,
        F: FnOnce(&V) -> T,
    {
        self.map.borrow().get(k).map(|s| f(s))
    }
}
#[cfg(test)]
mod test {
    use crate::utils::StableMap;

    #[test]
    fn get() {
        let mut map: StableMap<i32, i32> = StableMap::default();

        // Vacant entry is being initialized with a default.
        assert_eq!(map.inner().len(), 0);
        assert_eq!(map.get(1), &0);
        assert_eq!(map.inner().len(), 1);

        // Mutated value persisted.
        *map.get_mut(1) += 1;
        assert_eq!(map.get(1), &1);
    }

    #[test]
    fn get_mut() {
        let mut map: StableMap<i32, i32> = StableMap::default();

        // Vacant entry is being initialized with a default.
        assert_eq!(map.inner().len(), 0);
        assert_eq!(map.get_mut(1), &0);
        assert_eq!(map.inner().len(), 1);

        // Mutated value persisted.
        *map.get_mut(1) += 1;
        assert_eq!(map.get_mut(1), &1);

        // Vacant entry persisted as modified.
        assert_eq!(map.inner().len(), 1);
        *map.get_mut(2) += 1;
        assert_eq!(map.get_mut(2), &1);
        assert_eq!(map.inner().len(), 2);
    }

    #[test]
    fn map_value_ref() {
        // Returns none if the value does not exist.
        let mut map: StableMap<i32, i32> = StableMap::default();
        assert!(map.map_value_ref(&1, |v| v.is_negative()).is_none());

        // Successfully executes a callback if the value is found.
        *map.get_mut(1) -= 1;
        assert!(map.map_value_ref(&1, |v| v.is_negative()).unwrap());
    }
}
