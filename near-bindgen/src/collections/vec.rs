//! A contiguous growable array type with elements allocated on the trie.
//! Indexing is `O(d)`, where `d` is the depth of the trie, while iteration is `O(1)` amortized for
//! each iteration step.
use crate::{
    assert, storage_iter_next, storage_peek, storage_range, storage_read,
    storage_remove, storage_write,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::ops::RangeBounds;

#[derive(Serialize, Deserialize)]
pub struct Vec<T> {
    len: usize,
    id: String,
    element: PhantomData<T>,
}

impl<T: Serialize + DeserializeOwned> Default for Vec<T> {
    fn default() -> Self {
        Self::new(crate::next_trie_id())
    }
}

impl<T> Vec<T> {
    /// Get the prefix under which all items are stored.
    fn iterator_prefix(&self) -> String {
        format!("{}Element", self.id)
    }

    /// Converts element index to element id.
    fn index_to_key(&self, index: usize) -> String {
        format!("{}{:019}", self.iterator_prefix(), index)
    }

    /// Clears the vector, removing all values.
    pub fn clear(&mut self) {
        for i in 0..self.len() {
            let key = self.index_to_key(i);
            let key = key.as_bytes();
            unsafe {
                storage_remove(key.len() as _, key.as_ptr());
            }
        }
        self.set_len(0);
    }
    /// Returns the number of elements in the vector, also referred to as its 'length'.
    pub fn len(&self) -> usize {
        self.len
    }

    fn set_len(&mut self, value: usize) {
        self.len = value;
    }
}

impl<T: Serialize + DeserializeOwned> Vec<T> {
    /// Create new vector with zero size.
    pub fn new(id: String) -> Self {
        Self { id, element: PhantomData, len: 0 }
    }

    /// Removes and returns the element at position index within the vector, shifting all elements after it to the left.
    /// # Panics
    ///
    /// Panics if `index` is out of bounds.
    pub fn remove(&mut self, index: usize) -> T {
        let len = self.len();
        unsafe {
            assert(index < len);
        }
        let key = self.index_to_key(index);
        let key = key.as_bytes();
        let data = storage_read(key.len() as _, key.as_ptr());
        let result = bincode::deserialize(&data).ok().unwrap();
        unsafe {
            storage_remove(key.len() as _, key.as_ptr());
        }
        // Shift the elements to the left.
        for i in (index + 1)..len {
            let old_key = self.index_to_key(i);
            let old_key = old_key.as_bytes();
            let new_key = self.index_to_key(i - 1);
            let new_key = new_key.as_bytes();
            let data = storage_read(old_key.len() as _, old_key.as_ptr());
            unsafe {
                storage_write(new_key.len() as _, new_key.as_ptr(), data.len() as _, data.as_ptr());
            }
        }
        let last_key = self.index_to_key(len - 1);
        let last_key = last_key.as_bytes();
        unsafe {
            storage_remove(last_key.len() as _, last_key.as_ptr());
        }
        self.set_len(len - 1);
        result
    }

    /// Inserts an element at position `index` within the vector, shifting all
    /// elements after it to the right.
    ///
    /// # Panics
    ///
    /// Panics if `index > len`.
    pub fn insert(&mut self, index: usize, element: T) {
        let len = self.len();
        unsafe {
            assert(index <= len);
        }
        // Shift the elements to the right.
        for i in (index..len).rev() {
            let old_key = self.index_to_key(i);
            let old_key = old_key.as_bytes();
            let new_key = self.index_to_key(i + 1);
            let new_key = new_key.as_bytes();
            let data = storage_read(old_key.len() as _, old_key.as_ptr());
            unsafe {
                storage_write(new_key.len() as _, new_key.as_ptr(), data.len() as _, data.as_ptr());
            }
        }
        self.set_len(len + 1);

        let key = self.index_to_key(index);
        let key = key.as_bytes();
        let data = bincode::serialize(&element).unwrap();
        unsafe {
            storage_write(key.len() as _, key.as_ptr(), data.len() as _, data.as_ptr());
        }
    }

    /// Appends an element to the back of a collection.
    pub fn push(&mut self, value: T) {
        let len = self.len();
        self.set_len(len + 1);

        let key = self.index_to_key(len);
        let key = key.as_bytes();
        let data = bincode::serialize(&value).unwrap();
        unsafe {
            storage_write(key.len() as _, key.as_ptr(), data.len() as _, data.as_ptr());
        }
    }

    /// Returns element based on the index. If `index >= len` returns `None`.
    pub fn get(&self, index: usize) -> Option<T> {
        if index < self.len() {
            let key = self.index_to_key(index);
            let key = key.as_bytes();
            let data = storage_read(key.len() as _, key.as_ptr());
            bincode::deserialize(&data).ok()
        } else {
            None
        }
    }

    /// Removes the last element from a vector.
    pub fn pop(&mut self) {
        let len = self.len();
        self.set_len(len - 1);
        let key = self.index_to_key(len - 1);
        let key = key.as_bytes();
        unsafe {
            storage_remove(key.len() as _, key.as_ptr());
        }
    }

    /// Returns the first element of the slice, or `None` if it is empty.
    pub fn first(&self) -> Option<T> {
        self.get(0)
    }

    /// Returns the last element of the slice, or `None` if it is empty.
    pub fn last(&self) -> Option<T> {
        let len = self.len();
        if len > 0 {
            self.get(len - 1)
        } else {
            None
        }
    }

    /// Copies elements into an `std::vec::Vec`.
    pub fn to_vec(&self) -> std::vec::Vec<T> {
        let res = self.into_iter().collect();
        res
    }

    /// Creates a draining iterator that removes the specified range in the vector
    /// and yields the removed items.
    ///
    /// Note 1: The element range is removed even if the iterator is only
    /// partially consumed or not consumed at all.
    ///
    /// # Panics
    ///
    /// Panics if the starting point is greater than the end point or if
    /// the end point is greater than the length of the vector.
    pub fn drain<R>(&mut self, range: R) -> Drain<'_, T>
    where
        R: RangeBounds<usize>,
    {
        use std::ops::Bound::*;
        let len = self.len();
        let start = match range.start_bound() {
            Included(&n) => n,
            Excluded(&n) => n + 1,
            Unbounded => 0,
        };
        let end = match range.end_bound() {
            Included(&n) => n + 1,
            Excluded(&n) => n,
            Unbounded => len,
        };
        assert!(start <= end);
        assert!(end <= len);
        Drain { vec: self, start, end, curr: start }
    }
}

/// A draining iterator for `Vec<T>`.
///
/// This `struct` is created by the [`drain`] method on [`Vec`].
pub struct Drain<'a, T> {
    vec: &'a mut Vec<T>,
    start: usize,
    curr: usize,
    end: usize,
}

impl<T: Serialize + DeserializeOwned> Iterator for Drain<'_, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.curr == self.end {
            None
        } else {
            self.vec.get(self.curr).map(|el| {
                self.curr += 1;
                el
            })
        }
    }
}

impl<T> Drop for Drain<'_, T> {
    fn drop(&mut self) {
        for i in self.start..self.end {
            let key = self.vec.index_to_key(i);
            let key = key.as_bytes();
            unsafe {
                storage_remove(key.len() as _, key.as_ptr());
            }
        }

        let old_len = self.vec.len();

        // Shift right elements left.
        for i in self.end..old_len {
            let old_key = self.vec.index_to_key(i);
            let old_key = old_key.as_bytes();
            let new_key = self.vec.index_to_key(i - self.end + self.start);
            let new_key = new_key.as_bytes();
            let data = storage_read(old_key.len() as _, old_key.as_ptr());
            unsafe {
                storage_write(new_key.len() as _, new_key.as_ptr(), data.len() as _, data.as_ptr());
            }
        }

        // Remove old entries.
        for i in (old_len - 1 + self.end - self.start)..old_len {
            let key = self.vec.index_to_key(i);
            let key = key.as_bytes();
            unsafe {
                storage_remove(key.len() as _, key.as_ptr());
            }
        }
        self.vec.set_len(old_len - self.end + self.start);
    }
}

/// Creates a consuming iterator. The vector cannot be used after calling this.
impl<T: Serialize + DeserializeOwned> IntoIterator for Vec<T> {
    type Item = T;
    type IntoIter = IntoVec<T>;

    fn into_iter(self) -> Self::IntoIter {
        let start = self.index_to_key(0);
        let start = start.as_bytes();
        let end = self.index_to_key(self.len());
        let end = end.as_bytes();
        let iterator_id = unsafe {
            storage_range(start.len() as _, start.as_ptr(), end.len() as _, end.as_ptr())
        };
        IntoVec { iterator_id, vec: self, ended: false }
    }
}

/// Consuming iterator for `Vec<T>`.
pub struct IntoVec<T> {
    iterator_id: u32,
    vec: Vec<T>,
    ended: bool,
}

impl<T: Serialize + DeserializeOwned> Iterator for IntoVec<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.ended {
            return None;
        }
        let key_data = storage_peek(self.iterator_id);
        if key_data.is_empty() {
            return None;
        }
        let data = storage_read(key_data.len() as _, key_data.as_ptr());
        let ended = unsafe { storage_iter_next(self.iterator_id) } == 0;
        if ended {
            self.ended = true;
        }
        bincode::deserialize(&data).ok()
    }
}

impl<T> Drop for IntoVec<T> {
    fn drop(&mut self) {
        self.vec.clear();
    }
}

impl<'a, T: Serialize + DeserializeOwned> IntoIterator for &'a Vec<T> {
    type Item = T;
    type IntoIter = IntoVecRef<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        let start = self.index_to_key(0);
        let start = start.as_bytes();
        let end = self.index_to_key(self.len());
        let end = end.as_bytes();
        let iterator_id = unsafe {
            storage_range(start.len() as _, start.as_ptr(), end.len() as _, end.as_ptr())
        };
        IntoVecRef { iterator_id, vec: self, ended: false }
    }
}

impl<'a, T: Serialize + DeserializeOwned> IntoIterator for &'a mut Vec<T> {
    type Item = T;
    type IntoIter = IntoVecRef<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        let start = self.index_to_key(0);
        let start = start.as_bytes();
        let end = self.index_to_key(self.len());
        let end = end.as_bytes();
        let iterator_id = unsafe {
            storage_range(start.len() as _, start.as_ptr(), end.len() as _, end.as_ptr())
        };
        IntoVecRef { iterator_id, vec: self, ended: false }
    }
}

/// Non-consuming iterator for `Vec<T>`.
pub struct IntoVecRef<'a, T> {
    iterator_id: u32,
    #[allow(dead_code)]
    vec: &'a Vec<T>,
    ended: bool,
}

impl<'a, T: Serialize + DeserializeOwned> Iterator for IntoVecRef<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.ended {
            return None;
        }
        let key_data = storage_peek(self.iterator_id);
        if key_data.is_empty() {
            return None;
        }
        let data = storage_read(key_data.len() as _, key_data.as_ptr());
        let ended = unsafe { storage_iter_next(self.iterator_id) } == 0;
        if ended {
            self.ended = true;
        }
        bincode::deserialize(&data).ok()
    }
}

impl<T: Serialize + DeserializeOwned> Extend<T> for Vec<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        let mut len = self.len();
        for el in iter {
            let key = self.index_to_key(len);
            let key = key.as_bytes();
            let data = bincode::serialize(&el).unwrap();
            unsafe {
                storage_write(key.len() as _, key.as_ptr(), data.len() as _, data.as_ptr());
            }
            len += 1;
        }
        self.set_len(len);
    }
}
