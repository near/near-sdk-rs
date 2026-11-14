use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::panic::UnwindSafe;
use std::ptr::NonNull;

/// An owning heap pointer, like [`Box`], whose pointee is *not* asserted to be uniquely
/// referenced when the pointer itself is moved or passed by value.
///
/// Moving or passing a [`Box<T>`] by value retags it, re-asserting that it is the only pointer to
/// its `T`. Under Rust's aliasing model (Stacked/Tree Borrows) that invalidates any `&mut T`
/// previously derived from it. [`BTreeMap`] moves its values whenever a node splits or merges, so
/// a `&mut T` obtained from a `Box<T>` cached in the map becomes invalid as soon as an unrelated
/// key is inserted. This type stores a raw pointer instead: moving it is a plain copy that
/// leaves references to the pointee alone.
struct AliasableBox<T> {
    ptr: NonNull<T>,
    /// This type owns a `T` and drops it.
    _owns: PhantomData<T>,
}

impl<T> AliasableBox<T> {
    fn new(value: T) -> Self {
        Self { ptr: NonNull::from(Box::leak(Box::new(value))), _owns: PhantomData }
    }

    /// Raw pointer to the value. It stays valid, and keeps pointing at the same value, for as
    /// long as `self` (or wherever `self` is moved to) is alive.
    fn as_ptr(&self) -> NonNull<T> {
        self.ptr
    }
}

impl<T: Default> Default for AliasableBox<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T> Deref for AliasableBox<T> {
    type Target = T;
    fn deref(&self) -> &T {
        // SAFETY: `ptr` came from `Box::leak` and is freed only in `drop`, so it is valid and
        // initialized. A shared borrow of the owner allows shared access to the value.
        unsafe { self.ptr.as_ref() }
    }
}

impl<T> DerefMut for AliasableBox<T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: as in `deref`; a unique borrow of the owner allows unique access to the value.
        unsafe { self.ptr.as_mut() }
    }
}

impl<T> Drop for AliasableBox<T> {
    fn drop(&mut self) {
        // SAFETY: `ptr` came from `Box::leak` and ownership was never given away, so turning it
        // back into a `Box` here frees it exactly once.
        drop(unsafe { Box::from_raw(self.ptr.as_ptr()) });
    }
}

// SAFETY: `AliasableBox<T>` uniquely owns its `T`, exactly like `Box<T>`, so it can be sent or
// shared across threads under the same conditions. These impls (and `UnwindSafe`) keep the auto
// traits of the collections the same as when the cache held `Box<V>`.
unsafe impl<T: Send> Send for AliasableBox<T> {}
// SAFETY: see `Send` above.
unsafe impl<T: Sync> Sync for AliasableBox<T> {}
impl<T: UnwindSafe> UnwindSafe for AliasableBox<T> {}

/// This map is used as an append-only cache of keys to values. Insertions in the map can be done
/// with only an immutable reference: each value lives in its own heap allocation that is never
/// moved, reallocated or freed while the map is alive, and the map holds it through a raw pointer
/// ([`AliasableBox`]). Rebalancing the `BTreeMap` therefore moves only those pointers and never
/// touches, or asserts unique access to, a value that a caller holds a reference to.
pub(crate) struct StableMap<K, V> {
    map: RefCell<BTreeMap<K, AliasableBox<V>>>,
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
        let ptr = self.map.borrow_mut().entry(k).or_default().as_ptr();
        // SAFETY: here, we extend the lifetime of `V` from the local `RefCell` borrow to the
        // `&self`. The value is valid for that long: through `&self` the map is only ever appended
        // to, entries are never removed or replaced, and the value's allocation does not move
        // when the `BTreeMap` rebalances. The reference is derived from the raw pointer, not from
        // a `Box`, so later insertions don't invalidate it. Mutable access requires `&mut self`,
        // which cannot coexist with the returned borrow.
        unsafe { ptr.as_ref() }
    }
    /// Gets mutable reference to value if it exists in the map. If it does not exist, the default
    /// value will be used to initialize before returning a reference to it.
    ///
    /// The returned reference is derived from the value's own raw pointer, so it is not
    /// invalidated by later insertions of *other* keys (see [`AliasableBox`]). Callers that
    /// extend its lifetime (the `IterMut` impls of the `store` collections) rely on this, and
    /// must not create any other reference to the same key's value while it is live.
    pub(crate) fn get_mut(&mut self, k: K) -> &mut V
    where
        K: Ord,
        V: Default,
    {
        let mut ptr = self.map.get_mut().entry(k).or_default().as_ptr();
        // SAFETY: the pointer is valid (see `get`), and `&mut self` guarantees that no other
        // reference obtained through this map is live for the returned lifetime.
        unsafe { ptr.as_mut() }
    }
    /// Iterates over all cached entries in key order, with mutable access to the values.
    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = (&K, &mut V)> {
        self.map.get_mut().iter_mut().map(|(k, v)| (k, &mut **v))
    }
    #[cfg(test)]
    fn len(&mut self) -> usize {
        self.map.get_mut().len()
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
        assert_eq!(map.len(), 0);
        assert_eq!(map.get(1), &0);
        assert_eq!(map.len(), 1);

        // Mutated value persisted.
        *map.get_mut(1) += 1;
        assert_eq!(map.get(1), &1);
    }

    #[test]
    fn get_mut() {
        let mut map: StableMap<i32, i32> = StableMap::default();

        // Vacant entry is being initialized with a default.
        assert_eq!(map.len(), 0);
        assert_eq!(map.get_mut(1), &0);
        assert_eq!(map.len(), 1);

        // Mutated value persisted.
        *map.get_mut(1) += 1;
        assert_eq!(map.get_mut(1), &1);

        // Vacant entry persisted as modified.
        assert_eq!(map.len(), 1);
        *map.get_mut(2) += 1;
        assert_eq!(map.get_mut(2), &1);
        assert_eq!(map.len(), 2);
    }

    /// Pointers handed out by `get_mut` must stay valid while the cache keeps growing. The
    /// `IterMut` impls of the `store` collections rely on this when they extend the lifetime of
    /// each yielded `&mut V`. The writes before growth matter: under Tree Borrows a written
    /// (Active) reference is what a later move of the owning `Box` would invalidate. 200 keys
    /// split both leaf and internal nodes of the cache `BTreeMap`. Run under Miri:
    /// `MIRIFLAGS=-Zmiri-tree-borrows cargo miri test -p near-sdk --lib stable_map`.
    #[test]
    fn values_stay_valid_across_cache_growth() {
        let mut map: StableMap<u32, u32> = StableMap::default();
        let n = 200;
        let mut ptrs: Vec<*mut u32> = Vec::new();
        for k in 0..n {
            let p: *mut u32 = map.get_mut(k);
            // SAFETY: `p` points at the cached value for `k`; no other reference to it is live.
            unsafe { *p = k };
            ptrs.push(p);
        }
        for (k, p) in ptrs.iter().enumerate() {
            // SAFETY: values are never removed, and each key's value is only reached through
            // its own pointer here. The cache moving its nodes must not invalidate it.
            unsafe { **p += 1 };
            assert_eq!(*map.get(k as u32), k as u32 + 1);
        }

        // Shared references stay valid across insertions made through `&self`.
        let map: StableMap<u32, u32> = StableMap::default();
        let refs: Vec<&u32> = (0..n).map(|k| map.get(k)).collect();
        assert!(refs.iter().all(|v| **v == 0));
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
