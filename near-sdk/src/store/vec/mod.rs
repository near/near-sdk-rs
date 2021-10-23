mod impls;
mod iter;

use std::{
    fmt,
    ops::{Bound, Range, RangeBounds},
};

use borsh::{BorshDeserialize, BorshSerialize};

pub use self::iter::{Drain, Iter, IterMut};
use super::ERR_INCONSISTENT_STATE;
use crate::{env, IntoStorageKey};

use super::IndexMap;

const ERR_INDEX_OUT_OF_BOUNDS: &str = "Index out of bounds";

fn expect_consistent_state<T>(val: Option<T>) -> T {
    val.unwrap_or_else(|| env::panic_str(ERR_INCONSISTENT_STATE))
}

/// An iterable implementation of vector that stores its content on the trie. This implementation
/// will load and store values in the underlying storage lazily.
///
/// Uses the following map: index -> element. Because the data is sharded to avoid reading/writing
/// large chunks of data, the values cannot be accessed as a contiguous piece of memory.
///
/// This implementation will cache all changes and loads and only updates values that are changed
/// in storage after it's dropped through it's [`Drop`] implementation. These changes can be updated
/// in storage before the variable is dropped by using [`Vector::flush`]. During the lifetime of
/// this type, storage will only be read a maximum of one time per index and only written once per
/// index unless specifically flushed.
///
/// This type should be a drop in replacement for [`Vec`] in most cases and will provide contracts
/// a vector structure which scales much better as the contract data grows.
///
/// # Examples
/// ```
/// use near_sdk::store::Vector;
///
///# near_sdk::test_utils::test_env::setup();
/// let mut vec = Vector::new(b"a");
/// assert!(vec.is_empty());
///
/// vec.push(1);
/// vec.push(2);
///
/// assert_eq!(vec.len(), 2);
/// assert_eq!(vec[0], 1);
///
/// assert_eq!(vec.pop(), Some(2));
/// assert_eq!(vec.len(), 1);
///
/// vec[0] = 7;
/// assert_eq!(vec[0], 7);
///
/// vec.extend([1, 2, 3].iter().copied());
/// assert!(Iterator::eq(vec.into_iter(), [7, 1, 2, 3].iter()));
/// ```
pub struct Vector<T>
where
    T: BorshSerialize,
{
    len: u32,
    values: IndexMap<T>,
}

//? Manual implementations needed only because borsh derive is leaking field types
// https://github.com/near/borsh-rs/issues/41
impl<T> BorshSerialize for Vector<T>
where
    T: BorshSerialize,
{
    fn serialize<W: borsh::maybestd::io::Write>(
        &self,
        writer: &mut W,
    ) -> Result<(), borsh::maybestd::io::Error> {
        BorshSerialize::serialize(&self.len, writer)?;
        BorshSerialize::serialize(&self.values, writer)?;
        Ok(())
    }
}

impl<T> BorshDeserialize for Vector<T>
where
    T: BorshSerialize,
{
    fn deserialize(buf: &mut &[u8]) -> Result<Self, borsh::maybestd::io::Error> {
        Ok(Self {
            len: BorshDeserialize::deserialize(buf)?,
            values: BorshDeserialize::deserialize(buf)?,
        })
    }
}

impl<T> Vector<T>
where
    T: BorshSerialize,
{
    /// Returns the number of elements in the vector, also referred to as its size.
    /// This function returns a `u32` rather than the [`Vec`] equivalent of `usize` to have
    /// consistency between targets.
    pub fn len(&self) -> u32 {
        self.len
    }

    /// Returns `true` if the vector contains no elements.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Create new vector with zero elements. Prefixes storage accesss with the prefix provided.
    pub fn new<S>(prefix: S) -> Self
    where
        S: IntoStorageKey,
    {
        Self { len: 0, values: IndexMap::new(prefix) }
    }

    /// Removes all elements from the collection. This will remove all storage values for the
    /// length of the [`Vector`].
    pub fn clear(&mut self) {
        for i in 0..self.len {
            self.values.set(i, None);
        }
        self.len = 0;
    }

    /// Flushes the cache and writes all modified values to storage.
    pub fn flush(&mut self) {
        self.values.flush();
    }

    /// Sets a value at a given index to the value provided. This does not shift values after the
    /// index to the right.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds.
    pub fn set(&mut self, index: u32, value: T) {
        if index >= self.len() {
            env::panic_str(ERR_INDEX_OUT_OF_BOUNDS);
        }

        self.values.set(index, Some(value));
    }

    /// Appends an element to the back of the collection.
    ///
    /// # Panics
    ///
    /// Panics if new length exceeds `u32::MAX`
    pub fn push(&mut self, element: T) {
        let last_idx = self.len();
        self.len =
            self.len.checked_add(1).unwrap_or_else(|| env::panic_str(ERR_INDEX_OUT_OF_BOUNDS));
        self.set(last_idx, element)
    }
}

impl<T> Vector<T>
where
    T: BorshSerialize + BorshDeserialize,
{
    /// Returns the element by index or `None` if it is not present.
    pub fn get(&self, index: u32) -> Option<&T> {
        if index >= self.len() {
            return None;
        }
        self.values.get(index)
    }

    /// Returns a mutable reference to the element at the `index` provided.
    pub fn get_mut(&mut self, index: u32) -> Option<&mut T> {
        if index >= self.len {
            return None;
        }
        self.values.get_mut(index)
    }

    fn swap(&mut self, a: u32, b: u32) {
        if a >= self.len() || b >= self.len() {
            env::panic_str(ERR_INDEX_OUT_OF_BOUNDS);
        }

        self.values.swap(a, b);
    }

    /// Removes an element from the vector and returns it.
    /// The removed element is replaced by the last element of the vector.
    /// Does not preserve ordering, but is `O(1)`.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds.
    pub fn swap_remove(&mut self, index: u32) -> T {
        if self.is_empty() {
            env::panic_str(ERR_INDEX_OUT_OF_BOUNDS);
        }

        self.swap(index, self.len() - 1);
        expect_consistent_state(self.pop())
    }

    /// Removes the last element from a vector and returns it, or `None` if it is empty.
    pub fn pop(&mut self) -> Option<T> {
        let new_idx = self.len.checked_sub(1)?;
        let prev = self.values.get_mut_inner(new_idx).replace(None);
        self.len = new_idx;
        prev
    }

    /// Inserts a element at `index`, returns an evicted element.
    ///
    /// # Panics
    ///
    /// If `index` is out of bounds.
    // TODO determine if this should be stabilized, included for backwards compat with old version
    pub fn replace(&mut self, index: u32, element: T) -> T {
        if index >= self.len {
            env::panic_str(ERR_INDEX_OUT_OF_BOUNDS);
        }
        self.values.insert(index, element).unwrap()
    }

    /// Returns an iterator over the vector. This iterator will lazily load any values iterated
    /// over from storage.
    pub fn iter(&self) -> Iter<T> {
        Iter::new(self)
    }

    /// Returns an iterator over the [`Vector`] that allows modifying each value. This iterator
    /// will lazily load any values iterated over from storage.
    pub fn iter_mut(&mut self) -> IterMut<T> {
        IterMut::new(self)
    }

    /// Creates a draining iterator that removes the specified range in the vector
    /// and yields the removed items.
    ///
    /// When the iterator **is** dropped, all elements in the range are removed
    /// from the vector, even if the iterator was not fully consumed. If the
    /// iterator **is not** dropped (with [`mem::forget`] for example), the collection will be left
    /// in an inconsistent state.
    ///
    /// This will not panic on invalid ranges (`end > length` or `end < start`) and instead the
    /// iterator will just be empty.
    pub fn drain<R>(&mut self, range: R) -> Drain<T>
    where
        R: RangeBounds<u32>,
    {
        let start = match range.start_bound() {
            Bound::Excluded(i) => {
                i.checked_add(1).unwrap_or_else(|| env::panic_str(ERR_INDEX_OUT_OF_BOUNDS))
            }
            Bound::Included(i) => *i,
            Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            Bound::Excluded(i) => *i,
            Bound::Included(i) => {
                i.checked_add(1).unwrap_or_else(|| env::panic_str(ERR_INDEX_OUT_OF_BOUNDS))
            }
            Bound::Unbounded => self.len(),
        };

        // Note: don't need to do bounds check if end < start, will just return None when iterating
        // This will also cap the max length at the length of the vector.
        Drain::new(self, Range { start, end: core::cmp::min(end, self.len()) })
    }
}

impl<T> fmt::Debug for Vector<T>
where
    T: BorshSerialize + BorshDeserialize + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if cfg!(feature = "expensive-debug") {
            fmt::Debug::fmt(&self.iter().collect::<Vec<_>>(), f)
        } else {
            f.debug_struct("Vector")
                .field("len", &self.len)
                .field("prefix", &self.values.prefix)
                .finish()
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use arbitrary::{Arbitrary, Unstructured};
    use borsh::{BorshDeserialize, BorshSerialize};
    use rand::{Rng, RngCore, SeedableRng};

    use super::Vector;
    use crate::{store::IndexMap, test_utils::test_env::setup_free};

    #[test]
    fn test_push_pop() {
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(0);
        let mut vec = Vector::new(b"v".to_vec());
        let mut baseline = vec![];
        for _ in 0..500 {
            let value = rng.gen::<u64>();
            vec.push(value);
            baseline.push(value);
        }
        let actual: Vec<u64> = vec.iter().cloned().collect();
        assert_eq!(actual, baseline);
        for _ in 0..501 {
            assert_eq!(baseline.pop(), vec.pop());
        }
    }

    #[test]
    pub fn test_replace() {
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(1);
        let mut vec = Vector::new(b"v".to_vec());
        let mut baseline = vec![];
        for _ in 0..500 {
            let value = rng.gen::<u64>();
            vec.push(value);
            baseline.push(value);
        }
        for _ in 0..500 {
            let index = rng.gen::<u32>() % vec.len();
            let value = rng.gen::<u64>();
            let old_value0 = vec[index];
            let old_value1 = core::mem::replace(vec.get_mut(index).unwrap(), value);
            let old_value2 = baseline[index as usize];
            assert_eq!(old_value0, old_value1);
            assert_eq!(old_value0, old_value2);
            *baseline.get_mut(index as usize).unwrap() = value;
        }
        let actual: Vec<_> = vec.iter().cloned().collect();
        assert_eq!(actual, baseline);
    }

    #[test]
    pub fn test_swap_remove() {
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(2);
        let mut vec = Vector::new(b"v".to_vec());
        let mut baseline = vec![];
        for _ in 0..500 {
            let value = rng.gen::<u64>();
            vec.push(value);
            baseline.push(value);
        }
        for _ in 0..500 {
            let index = rng.gen::<u32>() % vec.len();
            let old_value0 = vec[index];
            let old_value1 = vec.swap_remove(index);
            let old_value2 = baseline[index as usize];
            let last_index = baseline.len() - 1;
            baseline.swap(index as usize, last_index);
            baseline.pop();
            assert_eq!(old_value0, old_value1);
            assert_eq!(old_value0, old_value2);
        }
        let actual: Vec<_> = vec.iter().cloned().collect();
        assert_eq!(actual, baseline);
    }

    #[test]
    pub fn test_clear() {
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(3);
        let mut vec = Vector::new(b"v".to_vec());
        for _ in 0..100 {
            for _ in 0..(rng.gen::<u64>() % 20 + 1) {
                let value = rng.gen::<u64>();
                vec.push(value);
            }
            assert!(!vec.is_empty());
            vec.clear();
            assert!(vec.is_empty());
        }
    }

    #[test]
    pub fn test_extend() {
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(0);
        let mut vec = Vector::new(b"v".to_vec());
        let mut baseline = vec![];
        for _ in 0..100 {
            let value = rng.gen::<u64>();
            vec.push(value);
            baseline.push(value);
        }

        for _ in 0..100 {
            let mut tmp = vec![];
            for _ in 0..=(rng.gen::<u64>() % 20 + 1) {
                let value = rng.gen::<u64>();
                tmp.push(value);
            }
            baseline.extend(tmp.clone());
            vec.extend(tmp.clone());
        }
        let actual: Vec<_> = vec.iter().cloned().collect();
        assert_eq!(actual, baseline);
    }

    #[test]
    fn test_debug() {
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(4);
        let prefix = b"v".to_vec();
        let mut vec = Vector::new(prefix.clone());
        let mut baseline = vec![];
        for _ in 0..10 {
            let value = rng.gen::<u64>();
            vec.push(value);
            baseline.push(value);
        }
        let actual: Vec<_> = vec.iter().cloned().collect();
        assert_eq!(actual, baseline);
        for _ in 0..5 {
            assert_eq!(baseline.pop(), vec.pop());
        }
        if cfg!(feature = "expensive-debug") {
            assert_eq!(format!("{:#?}", vec), format!("{:#?}", baseline));
        } else {
            assert_eq!(
                format!("{:?}", vec),
                format!("Vector {{ len: 5, prefix: {:?} }}", vec.values.prefix)
            );
        }

        // * The storage is reused in the second part of this test, need to flush
        vec.flush();

        use borsh::{BorshDeserialize, BorshSerialize};
        #[derive(Debug, BorshSerialize, BorshDeserialize)]
        struct TestType(u64);

        let deserialize_only_vec =
            Vector::<TestType> { len: vec.len(), values: IndexMap::new(prefix) };
        let baseline: Vec<_> = baseline.into_iter().map(TestType).collect();
        if cfg!(feature = "expensive-debug") {
            assert_eq!(format!("{:#?}", deserialize_only_vec), format!("{:#?}", baseline));
        } else {
            assert_eq!(
                format!("{:?}", deserialize_only_vec),
                format!("Vector {{ len: 5, prefix: {:?} }}", deserialize_only_vec.values.prefix)
            );
        }
    }

    #[test]
    pub fn iterator_checks() {
        let mut vec = Vector::new(b"v");
        let mut baseline = vec![];
        for i in 0..10 {
            vec.push(i);
            baseline.push(i);
        }

        let mut vec_iter = vec.iter();
        let mut bl_iter = baseline.iter();
        assert_eq!(vec_iter.next(), bl_iter.next());
        assert_eq!(vec_iter.next_back(), bl_iter.next_back());
        assert_eq!(vec_iter.nth(3), bl_iter.nth(3));
        assert_eq!(vec_iter.nth_back(2), bl_iter.nth_back(2));

        // Check to make sure indexing overflow is handled correctly
        assert!(vec_iter.nth(5).is_none());
        assert!(bl_iter.nth(5).is_none());

        assert!(vec_iter.next().is_none());
        assert!(bl_iter.next().is_none());

        // Count check
        assert_eq!(vec.iter().count(), baseline.len());
    }

    #[test]
    fn drain_iterator() {
        let mut vec = Vector::new(b"v");
        let mut baseline = vec![0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        vec.extend(baseline.clone());

        assert!(Iterator::eq(vec.drain(1..=3), baseline.drain(1..=3)));
        assert_eq!(vec.iter().copied().collect::<Vec<_>>(), vec![0, 4, 5, 6, 7, 8, 9]);

        // Test incomplete drain
        {
            let mut drain = vec.drain(0..3);
            let mut b_drain = baseline.drain(0..3);
            assert_eq!(drain.next(), b_drain.next());
            assert_eq!(drain.next(), b_drain.next());
        }

        // 7 elements, drained 3
        assert_eq!(vec.len(), 4);

        // Test incomplete drain over limit
        {
            let mut drain = vec.drain(2..);
            let mut b_drain = baseline.drain(2..);
            assert_eq!(drain.next(), b_drain.next());
        }

        // Drain rest
        assert!(Iterator::eq(vec.drain(..), baseline.drain(..)));

        // Test double ended iterator functions
        let mut vec = Vector::new(b"v");
        let mut baseline = vec![0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        vec.extend(baseline.clone());

        {
            let mut drain = vec.drain(1..8);
            let mut b_drain = baseline.drain(1..8);
            assert_eq!(drain.nth(1), b_drain.nth(1));
            assert_eq!(drain.nth_back(2), b_drain.nth_back(2));
            assert_eq!(drain.len(), b_drain.len());
        }

        assert_eq!(vec.len() as usize, baseline.len());
        assert!(Iterator::eq(vec.iter(), baseline.iter()));

        assert!(Iterator::eq(vec.drain(..), baseline.drain(..)));
        crate::mock::with_mocked_blockchain(|m| assert!(m.take_storage().is_empty()));
    }

    #[derive(Arbitrary, Debug)]
    enum Op {
        Push(u8),
        Pop,
        Set(u32, u8),
        Remove(u32),
        Flush,
        Reset,
        Get(u32),
        Swap(u32, u32),
    }

    #[test]
    fn arbitrary() {
        setup_free();

        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(0);
        let mut buf = vec![0; 4096];
        for _ in 0..1024 {
            // Clear storage in-between runs
            crate::mock::with_mocked_blockchain(|b| b.take_storage());
            rng.fill_bytes(&mut buf);

            let mut sv = Vector::new(b"v");
            let mut mv = Vec::new();
            let u = Unstructured::new(&buf);
            if let Ok(ops) = Vec::<Op>::arbitrary_take_rest(u) {
                for op in ops {
                    match op {
                        Op::Push(v) => {
                            sv.push(v);
                            mv.push(v);
                            assert_eq!(sv.len() as usize, mv.len());
                        }
                        Op::Pop => {
                            assert_eq!(sv.pop(), mv.pop());
                            assert_eq!(sv.len() as usize, mv.len());
                        }
                        Op::Set(k, v) => {
                            if sv.is_empty() {
                                continue;
                            }
                            let k = k % sv.len();

                            sv.set(k, v);
                            mv[k as usize] = v;

                            // Extra get just to make sure set happened correctly
                            assert_eq!(sv[k], mv[k as usize]);
                        }
                        Op::Remove(i) => {
                            if sv.is_empty() {
                                continue;
                            }
                            let i = i % sv.len();
                            let r1 = sv.swap_remove(i);
                            let r2 = mv.swap_remove(i as usize);
                            assert_eq!(r1, r2);
                            assert_eq!(sv.len() as usize, mv.len());
                        }
                        Op::Flush => {
                            sv.flush();
                        }
                        Op::Reset => {
                            let serialized = sv.try_to_vec().unwrap();
                            sv = Vector::deserialize(&mut serialized.as_slice()).unwrap();
                        }
                        Op::Get(k) => {
                            let r1 = sv.get(k);
                            let r2 = mv.get(k as usize);
                            assert_eq!(r1, r2)
                        }
                        Op::Swap(i1, i2) => {
                            if sv.is_empty() {
                                continue;
                            }
                            let i1 = i1 % sv.len();
                            let i2 = i2 % sv.len();
                            sv.swap(i1, i2);
                            mv.swap(i1 as usize, i2 as usize)
                        }
                    }
                }
            }

            // After all operations, compare both vectors
            assert!(Iterator::eq(sv.iter(), mv.iter()));
        }
    }

    #[test]
    fn serialized_bytes() {
        use borsh::{BorshDeserialize, BorshSerialize};

        let mut vec = Vector::new(b"v".to_vec());
        vec.push("Some data");
        let serialized = vec.try_to_vec().unwrap();

        // Expected to serialize len then prefix
        let mut expected_buf = Vec::new();
        1u32.serialize(&mut expected_buf).unwrap();
        (b"v".to_vec()).serialize(&mut expected_buf).unwrap();

        assert_eq!(serialized, expected_buf);
        drop(vec);
        let vec = Vector::<String>::deserialize(&mut serialized.as_slice()).unwrap();
        assert_eq!(vec[0], "Some data");
    }
}
