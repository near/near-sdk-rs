mod entry;
mod impls;
mod iter;

use super::lookup_map as lm;
use crate::store::free_list::{FreeList, FreeListIndex};
use crate::store::key::{Sha256, ToKey};
use crate::store::LookupMap;
use crate::{env, IntoStorageKey};
use borsh::{BorshDeserialize, BorshSerialize};
pub use entry::Entry;
pub use iter::{Iter, IterMut, Keys, Range, RangeMut, Values, ValuesMut};
use std::borrow::Borrow;
use std::fmt;
use std::ops::RangeBounds;

use near_sdk_macros::near;

type NodeAndIndex<'a, K> = (FreeListIndex, &'a Node<K>);

fn expect<T>(val: Option<T>) -> T {
    val.unwrap_or_else(|| env::abort())
}

/// TreeMap based on AVL-tree
///
/// Runtime complexity (worst case):
/// - `get`/`contains_key`:     O(1) - LookupMap lookup
/// - `insert`/`remove`:        O(log(N))
/// - `min`/`max`:              O(log(N))
/// - `above`/`below`:          O(log(N))
/// - `range` of K elements:    O(Klog(N))
#[near(inside_nearsdk)]
pub struct TreeMap<K, V, H = Sha256>
where
    K: BorshSerialize + Ord,
    V: BorshSerialize,
    H: ToKey,
{
    // ser/de is independent of `K`, `V`, `H` ser/de, `BorshSerialize`/`BorshDeserialize`/`BorshSchema` bounds removed
    #[cfg_attr(not(feature = "abi"), borsh(bound(serialize = "", deserialize = "")))]
    #[cfg_attr(
        feature = "abi",
        borsh(bound(serialize = "", deserialize = ""), schema(params = ""))
    )]
    values: LookupMap<K, V, H>,
    // ser/de is independent of `K` ser/de, `BorshSerialize`/`BorshDeserialize`/`BorshSchema` bounds removed
    #[cfg_attr(not(feature = "abi"), borsh(bound(serialize = "", deserialize = "")))]
    #[cfg_attr(
        feature = "abi",
        borsh(bound(serialize = "", deserialize = ""), schema(params = ""))
    )]
    tree: Tree<K>,
}

impl<K, V, H> Drop for TreeMap<K, V, H>
where
    K: BorshSerialize + Ord,
    V: BorshSerialize,
    H: ToKey,
{
    fn drop(&mut self) {
        self.flush()
    }
}

impl<K, V, H> fmt::Debug for TreeMap<K, V, H>
where
    K: Ord + Clone + fmt::Debug + BorshSerialize + BorshDeserialize,
    V: fmt::Debug + BorshSerialize + BorshDeserialize,
    H: ToKey,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TreeMap")
            .field("root", &self.tree.root)
            .field("tree", &self.tree.nodes)
            .finish()
    }
}

#[near(inside_nearsdk)]
struct Tree<K>
where
    K: BorshSerialize,
{
    root: Option<FreeListIndex>,
    // ser/de is independent of `K` ser/de, `BorshSerialize`/`BorshDeserialize`/`BorshSchema` bounds removed
    #[cfg_attr(not(feature = "abi"), borsh(bound(serialize = "", deserialize = "")))]
    #[cfg_attr(
        feature = "abi",
        borsh(bound(serialize = "", deserialize = ""), schema(params = ""))
    )]
    nodes: FreeList<Node<K>>,
}

impl<K> Tree<K>
where
    K: BorshSerialize + Ord,
{
    fn new<S>(prefix: S) -> Self
    where
        S: IntoStorageKey,
    {
        Tree { root: None, nodes: FreeList::new(prefix) }
    }
}

#[near(inside_nearsdk)]
#[derive(Clone, Debug)]
struct Node<K> {
    key: K,                     // key stored in a node
    lft: Option<FreeListIndex>, // left link of a node
    rgt: Option<FreeListIndex>, // right link of a node
    ht: u32,                    // height of a subtree at a node
}

impl<K> Node<K>
where
    K: BorshSerialize + BorshDeserialize,
{
    fn of(key: K) -> Self {
        Self { key, lft: None, rgt: None, ht: 1 }
    }

    fn left<'a>(&self, list: &'a FreeList<Node<K>>) -> Option<(FreeListIndex, &'a Node<K>)> {
        self.lft.and_then(|id| list.get(id).map(|node| (id, node)))
    }

    fn right<'a>(&self, list: &'a FreeList<Node<K>>) -> Option<(FreeListIndex, &'a Node<K>)> {
        self.rgt.and_then(|id| list.get(id).map(|node| (id, node)))
    }
}

impl<K, V> TreeMap<K, V, Sha256>
where
    K: BorshSerialize + Ord,
    V: BorshSerialize,
{
    /// Initialize new [`TreeMap`] with the prefix provided.
    ///
    /// This prefix can be anything that implements [`IntoStorageKey`]. The prefix is used when
    /// storing and looking up values in storage to ensure no collisions with other collections.
    pub fn new<S>(prefix: S) -> Self
    where
        S: IntoStorageKey,
    {
        Self::with_hasher(prefix)
    }
}

impl<K, V, H> TreeMap<K, V, H>
where
    K: BorshSerialize + Ord,
    V: BorshSerialize,
    H: ToKey,
{
    pub fn with_hasher<S>(prefix: S) -> Self
    where
        S: IntoStorageKey,
    {
        let mut vec_key = prefix.into_storage_key();
        let map_key = [vec_key.as_slice(), b"v"].concat();
        vec_key.push(b'n');
        Self { values: LookupMap::with_hasher(map_key), tree: Tree::new(vec_key) }
    }

    /// Return the amount of elements inside of the map.
    pub fn len(&self) -> u32 {
        self.tree.nodes.len()
    }

    /// Returns true if there are no elements inside of the map.
    pub fn is_empty(&self) -> bool {
        self.tree.nodes.is_empty()
    }
}

impl<K, V, H> TreeMap<K, V, H>
where
    K: Ord + Clone + BorshSerialize,
    V: BorshSerialize + BorshDeserialize,
    H: ToKey,
{
    /// Clears the map, removing all key-value pairs. Keeps the allocated memory
    /// for reuse.
    pub fn clear(&mut self)
    where
        K: BorshDeserialize,
    {
        self.tree.root = None;
        for k in self.tree.nodes.drain() {
            // Set instead of remove to avoid loading the value from storage.
            self.values.set(k.key, None);
        }
    }

    /// Returns `true` if the map contains a value for the specified key.
    ///
    /// The key may be any borrowed form of the map's key type, but
    /// [`BorshSerialize`] and [`ToOwned<Owned = K>`](ToOwned) on the borrowed form *must* match
    /// those for the key type.
    pub fn contains_key<Q: ?Sized>(&self, k: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: BorshSerialize + ToOwned<Owned = K> + Ord,
    {
        self.values.contains_key(k)
    }

    /// Returns a reference to the value corresponding to the key.
    ///
    /// The key may be any borrowed form of the map's key type, but
    /// [`BorshSerialize`] and [`ToOwned<Owned = K>`](ToOwned) on the borrowed form *must* match
    /// those for the key type.
    pub fn get<Q: ?Sized>(&self, k: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: BorshSerialize + ToOwned<Owned = K>,
    {
        self.values.get(k)
    }

    /// Returns the key-value pair corresponding to the supplied key.
    ///
    /// The supplied key may be any borrowed form of the map's key type, but the ordering
    /// on the borrowed form *must* match the ordering on the key type.
    ///
    /// # Examples
    ///
    /// ```
    /// use near_sdk::store::TreeMap;
    ///
    /// let mut map = TreeMap::new(b"t");
    /// map.insert(1, "a".to_string());
    /// assert_eq!(map.get_key_value(&1), Some((&1, &"a".to_string())));
    /// assert_eq!(map.get_key_value(&2), None);
    /// ```
    pub fn get_key_value<Q: ?Sized>(&self, k: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q> + BorshDeserialize,
        Q: BorshSerialize + ToOwned<Owned = K> + Ord,
    {
        self.values.get(k).map(|v| (expect(self.tree.equal_key(k)), v))
    }

    /// Returns a mutable reference to the value corresponding to the key.
    ///
    /// The key may be any borrowed form of the map's key type, but
    /// [`BorshSerialize`] and [`ToOwned<Owned = K>`](ToOwned) on the borrowed form *must* match
    /// those for the key type.
    pub fn get_mut<Q: ?Sized>(&mut self, k: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: BorshSerialize + ToOwned<Owned = K>,
    {
        self.values.get_mut(k)
    }

    /// Inserts a key-value pair into the map.
    ///
    /// If the map did not have this key present, [`None`] is returned.
    ///
    /// If the map did have this key present, the value is updated, and the old
    /// value is returned. The key is not updated, though; this matters for
    /// types that can be `==` without being identical.
    pub fn insert(&mut self, key: K, value: V) -> Option<V>
    where
        K: Clone + BorshDeserialize,
    {
        // fix pattern when refactor
        match self.values.entry(key.clone()) {
            lm::Entry::Occupied(mut v) => Some(core::mem::replace(v.get_mut(), value)),
            lm::Entry::Vacant(v) => {
                self.tree.internal_insert(key);
                v.insert(value);
                None
            }
        }
    }

    /// Removes a key from the map, returning the value at the key if the key
    /// was previously in the map.
    ///
    /// The key may be any borrowed form of the map's key type, but
    /// [`BorshSerialize`] and [`ToOwned<Owned = K>`](ToOwned) on the borrowed form *must* match
    /// those for the key type.
    pub fn remove<Q: ?Sized>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q> + BorshDeserialize,
        Q: BorshSerialize + ToOwned<Owned = K> + Ord,
    {
        self.remove_entry(key).map(|(_, v)| v)
    }
}

enum Edge {
    Left,
    Right,
}

impl<K> Tree<K>
where
    K: Ord + BorshSerialize + BorshDeserialize,
{
    fn node(&self, id: FreeListIndex) -> Option<&Node<K>> {
        self.nodes.get(id)
    }

    /// Returns the smallest key that is strictly greater than key given as the parameter
    fn higher<Q>(&self, key: &Q) -> Option<&K>
    where
        K: Borrow<Q>,
        Q: ?Sized + Ord,
    {
        let root = self.root?;
        self.above_at(root, key)
    }

    /// Returns the largest key that is strictly less than key given as the parameter
    fn lower<Q>(&self, key: &Q) -> Option<&K>
    where
        K: Borrow<Q>,
        Q: ?Sized + Ord,
    {
        let root = self.root?;
        self.below_at(root, key)
    }

    fn equal_key<Q>(&self, key: &Q) -> Option<&K>
    where
        K: Borrow<Q>,
        Q: ?Sized + Ord,
    {
        self.root.map(|root| self.equal_at(root, key)).unwrap_or_default()
    }

    fn floor_key<Q>(&self, key: &Q) -> Option<&K>
    where
        K: Borrow<Q>,
        Q: ?Sized + Ord,
    {
        if let Some(key) = self.equal_key(key) {
            Some(key)
        } else {
            self.lower(key)
        }
    }

    fn ceil_key<Q>(&self, key: &Q) -> Option<&K>
    where
        K: Borrow<Q>,
        Q: ?Sized + Ord,
    {
        if let Some(key) = self.equal_key(key) {
            Some(key)
        } else {
            self.higher(key)
        }
    }

    /// Returns (node, parent node) of left-most lower (min) node starting from given node `at`.
    fn min_at(&self, mut at: FreeListIndex) -> Option<(NodeAndIndex<K>, Option<NodeAndIndex<K>>)> {
        let mut parent: Option<NodeAndIndex<K>> = None;
        loop {
            let node = self.node(at);
            match node.and_then(|n| n.lft) {
                Some(lft) => {
                    parent = Some((at, expect(node)));
                    at = lft;
                }
                None => {
                    return node.map(|node| ((at, node), parent));
                }
            }
        }
    }

    /// Returns (node, parent node) of right-most lower (max) node starting from given node `at`.
    fn max_at(&self, mut at: FreeListIndex) -> Option<(NodeAndIndex<K>, Option<NodeAndIndex<K>>)> {
        let mut parent: Option<NodeAndIndex<K>> = None;
        loop {
            let node = self.node(at);
            match node.and_then(|n| n.rgt) {
                Some(rgt) => {
                    parent = Some((at, expect(node)));
                    at = rgt;
                }
                None => {
                    return node.map(|node| ((at, node), parent));
                }
            }
        }
    }

    fn above_at<Q>(&self, mut at: FreeListIndex, key: &Q) -> Option<&K>
    where
        K: Borrow<Q>,
        Q: ?Sized + Ord,
    {
        let mut seen: Option<&K> = None;
        while let Some(node) = self.node(at) {
            let k: &Q = node.key.borrow();
            if k.le(key) {
                match node.rgt {
                    Some(rgt) => at = rgt,
                    None => break,
                }
            } else {
                seen = Some(&node.key);
                match node.lft {
                    Some(lft) => at = lft,
                    None => break,
                }
            }
        }
        seen
    }

    fn below_at<Q>(&self, mut at: FreeListIndex, key: &Q) -> Option<&K>
    where
        K: Borrow<Q>,
        Q: ?Sized + Ord,
    {
        let mut seen: Option<&K> = None;
        while let Some(node) = self.node(at) {
            let k: &Q = node.key.borrow();
            if k.lt(key) {
                seen = Some(&node.key);
                match node.rgt {
                    Some(rgt) => at = rgt,
                    None => break,
                }
            } else {
                match node.lft {
                    Some(lft) => at = lft,
                    None => break,
                }
            }
        }
        seen
    }

    fn equal_at<Q>(&self, mut at: FreeListIndex, key: &Q) -> Option<&K>
    where
        K: Borrow<Q>,
        Q: ?Sized + Ord,
    {
        while let Some(node) = self.node(at) {
            let k: &Q = node.key.borrow();
            if k.eq(key) {
                return Some(&node.key);
            } else if k.lt(key) {
                match node.rgt {
                    Some(rgt) => at = rgt,
                    None => break,
                }
            } else {
                match node.lft {
                    Some(lft) => at = lft,
                    None => break,
                }
            }
        }
        None
    }

    /// Returns node and parent node and respective metadata for a node that holds the `key`.
    /// For root node, `None` is returned for the parent and metadata.
    /// The metadata included in the result includes the indices for the node and parent, as well
    /// as which edge the found node is of the parent, if one.
    #[allow(clippy::type_complexity)]
    fn lookup_at<Q: ?Sized>(
        &self,
        mut at: FreeListIndex,
        key: &Q,
    ) -> Option<(NodeAndIndex<K>, Option<(FreeListIndex, &Node<K>, Edge)>)>
    where
        K: Borrow<Q>,
        Q: BorshSerialize + Eq + PartialOrd,
    {
        let mut p = None;
        let mut curr = Some(expect(self.node(at)));
        while let Some(node) = curr {
            let node_key: &Q = node.key.borrow();
            if node_key.eq(key) {
                return Some(((at, node), p));
            } else if node_key.lt(key) {
                match node.rgt {
                    Some(rgt) => {
                        p = Some((at, node, Edge::Right));
                        at = rgt;
                    }
                    None => break,
                }
            } else {
                match node.lft {
                    Some(lft) => {
                        p = Some((at, node, Edge::Left));
                        at = lft;
                    }
                    None => break,
                }
            }
            curr = self.node(at);
        }
        None
    }
}

impl<K> Tree<K>
where
    K: Ord + BorshSerialize + BorshDeserialize + Clone,
{
    fn internal_insert(&mut self, key: K) {
        if let Some(root) = self.root {
            let node = expect(self.node(root)).clone();
            self.root = Some(self.insert_at(node, root, key));
        } else {
            self.root = Some(self.nodes.insert(Node::of(key)));
        }
    }

    fn insert_at(&mut self, mut node: Node<K>, id: FreeListIndex, key: K) -> FreeListIndex {
        if key.eq(&node.key) {
            // This branch should not be hit, because we check for existence in insert.
            id
        } else {
            if key.lt(&node.key) {
                let idx = match node.lft {
                    Some(lft) => self.insert_at(expect(self.node(lft)).clone(), lft, key),
                    None => self.nodes.insert(Node::of(key)),
                };
                node.lft = Some(idx);
            } else {
                let idx = match node.rgt {
                    Some(rgt) => self.insert_at(expect(self.node(rgt)).clone(), rgt, key),
                    None => self.nodes.insert(Node::of(key)),
                };
                node.rgt = Some(idx);
            };

            self.update_height(&mut node, id);
            self.enforce_balance(&mut node, id)
        }
    }

    // Calculate and save the height of a subtree at node `at`:
    // height[at] = 1 + max(height[at.L], height[at.R])
    fn update_height(&mut self, node: &mut Node<K>, id: FreeListIndex) {
        let lft = node.lft.and_then(|id| self.node(id).map(|n| n.ht)).unwrap_or_default();
        let rgt = node.rgt.and_then(|id| self.node(id).map(|n| n.ht)).unwrap_or_default();

        node.ht = 1 + std::cmp::max(lft, rgt);
        // This side effect isn't great, but a lot of logic depends on values in storage/cache to be
        // up to date. Until changes and the tree are kept all in a single data structure, this
        // will be necessary.
        *expect(self.nodes.get_mut(id)) = node.clone();
    }

    // Balance = difference in heights between left and right subtrees at given node.
    fn get_balance(&self, node: &Node<K>) -> i64 {
        let lht = node.lft.and_then(|id| self.node(id).map(|n| n.ht)).unwrap_or_default();
        let rht = node.rgt.and_then(|id| self.node(id).map(|n| n.ht)).unwrap_or_default();

        lht as i64 - rht as i64
    }

    // Left rotation of an AVL subtree with at node `at`.
    // New root of subtree is returned, caller is responsible for updating proper link from parent.
    fn rotate_left(&mut self, node: &mut Node<K>, id: FreeListIndex) -> FreeListIndex {
        let (left_id, mut left) = expect(node.left(&self.nodes).map(|(id, n)| (id, n.clone())));
        let lft_rgt = left.rgt;

        // at.L = at.L.R
        node.lft = lft_rgt;

        // at.L.R = at
        left.rgt = Some(id);

        // at = at.L
        self.update_height(node, id);
        self.update_height(&mut left, left_id);

        left_id
    }

    // Right rotation of an AVL subtree at node in `at`.
    // New root of subtree is returned, caller is responsible for updating proper link from parent.
    fn rotate_right(&mut self, node: &mut Node<K>, id: FreeListIndex) -> FreeListIndex {
        let (rgt_id, mut rgt) = expect(node.right(&self.nodes).map(|(id, r)| (id, r.clone())));
        let rgt_lft = rgt.lft;

        // at.R = at.R.L
        node.rgt = rgt_lft;

        // at.R.L = at
        rgt.lft = Some(id);

        // at = at.R
        self.update_height(node, id);
        self.update_height(&mut rgt, rgt_id);

        rgt_id
    }

    // Check balance at a given node and enforce it if necessary with respective rotations.
    fn enforce_balance(&mut self, node: &mut Node<K>, id: FreeListIndex) -> FreeListIndex {
        let balance = self.get_balance(node);
        if balance > 1 {
            let (left_id, mut left) = expect(node.left(&self.nodes).map(|(id, n)| (id, n.clone())));
            if self.get_balance(&left) < 0 {
                let rotated = self.rotate_right(&mut left, left_id);
                node.lft = Some(rotated);
            }
            self.rotate_left(node, id)
        } else if balance < -1 {
            let (right_id, mut right) =
                expect(node.right(&self.nodes).map(|(id, r)| (id, r.clone())));
            if self.get_balance(&right) > 0 {
                let rotated = self.rotate_left(&mut right, right_id);
                node.rgt = Some(rotated);
            }
            self.rotate_right(node, id)
        } else {
            id
        }
    }

    // Navigate from root to node holding `key` and backtrace back to the root
    // enforcing balance (if necessary) along the way.
    fn check_balance(&mut self, at: FreeListIndex, key: &K) -> FreeListIndex {
        match self.node(at).cloned() {
            Some(mut node) => {
                if !node.key.eq(key) {
                    if node.key.gt(key) {
                        if let Some(l) = node.lft {
                            let id = self.check_balance(l, key);
                            node.lft = Some(id);
                        }
                    } else if let Some(r) = node.rgt {
                        let id = self.check_balance(r, key);
                        node.rgt = Some(id);
                    }
                }
                self.update_height(&mut node, at);
                self.enforce_balance(&mut node, at)
            }
            None => at,
        }
    }

    // Node holding the key is not removed from the tree - instead the substitute node is found,
    // the key is copied to 'removed' node from substitute node, and then substitute node gets
    // removed from the tree.
    //
    // The substitute node is either:
    // - right-most (max) node of the left subtree (containing smaller keys) of node holding `key`
    // - or left-most (min) node of the right subtree (containing larger keys) of node holding `key`
    //
    fn do_remove<Q: ?Sized>(&mut self, key: &Q) -> Option<K>
    where
        K: Borrow<Q>,
        Q: BorshSerialize + Eq + PartialOrd,
    {
        // r_node - node containing key of interest
        // remove_parent - immediate parent node of r_node
        let ((r_id, mut r_node), remove_parent) = match self
            .root
            .and_then(|root| self.lookup_at(root, key))
        {
            Some(((l_id, node), r)) => ((l_id, node.clone()), r.map(|(i, n, e)| (i, n.clone(), e))),
            None => return None, // cannot remove a missing key, no changes to the tree needed
        };

        let lft_opt = r_node.lft;
        let rgt_opt = r_node.rgt;

        if lft_opt.is_none() && rgt_opt.is_none() {
            // Node is leaf, can simply remove and rebalance.
            if let Some((p_id, mut p_node, p_edge)) = remove_parent {
                match p_edge {
                    Edge::Right => {
                        p_node.rgt = None;
                    }
                    Edge::Left => {
                        p_node.lft = None;
                    }
                }
                self.update_height(&mut p_node, p_id);

                // removing node might have caused a imbalance - balance the tree up to the root,
                // starting from lowest affected key - the parent of a leaf node in this case.
                // At this point, we can assume there is a root because there is at least the parent
                self.root = self.root.map(|root| self.check_balance(root, &p_node.key));
            }

            let removed = expect(self.nodes.remove(r_id));
            if Some(r_id) == self.root {
                self.root = None;
            }

            Some(removed.key)
        } else {
            // non-leaf node, select subtree to proceed with depending on balance
            let b = self.get_balance(&r_node);
            if b >= 0 {
                // proceed with left subtree
                let left = expect(lft_opt);

                // k - min key from left subtree
                // n - node that holds key k, p - immediate parent of n
                let ((min_id, _), parent) = expect(self.max_at(left));
                let mut parent = parent.map(|(i, n)| (i, n.clone()));

                let replaced_key = if let Some((p_id, parent_node)) = &mut parent {
                    // Min has a parent, attach its left node to the parent before moving
                    let min_left = expect(self.nodes.remove(min_id));

                    parent_node.rgt = min_left.lft;

                    let r_key = core::mem::replace(&mut r_node.key, min_left.key);
                    self.update_height(parent_node, *p_id);
                    *expect(self.nodes.get_mut(r_id)) = r_node.clone();
                    r_key
                } else {
                    let max_left = expect(self.nodes.remove(min_id));

                    // Update link and move key into removal node location
                    r_node.lft = max_left.lft;

                    let r_key = core::mem::replace(&mut r_node.key, max_left.key);
                    self.update_height(&mut r_node, r_id);
                    r_key
                };

                // removing node might have caused an imbalance - balance the tree up to the root,
                // starting from the lowest affected key (max key from left subtree in this case)
                self.root = self.root.map(|root| {
                    self.check_balance(
                        root,
                        parent.as_ref().map(|p| &p.1.key).unwrap_or(&r_node.key),
                    )
                });
                Some(replaced_key)
            } else {
                // proceed with right subtree
                let rgt = expect(rgt_opt);

                // k - min key from right subtree
                // n - node that holds key k, p - immediate parent of n
                let ((min_id, _), parent) = expect(self.min_at(rgt));
                let mut parent = parent.map(|(i, n)| (i, n.clone()));

                let replaced_key = if let Some((p_id, parent_node)) = &mut parent {
                    // Min has a parent, attach its right node to the parent before moving
                    let min_right = expect(self.nodes.remove(min_id));

                    parent_node.lft = min_right.rgt;

                    let r_key = core::mem::replace(&mut r_node.key, min_right.key);
                    self.update_height(parent_node, *p_id);
                    *expect(self.nodes.get_mut(r_id)) = r_node.clone();
                    r_key
                } else {
                    let min_right = expect(self.nodes.remove(min_id));

                    // Update link and move key into removal node location
                    r_node.rgt = min_right.rgt;

                    let r_key = core::mem::replace(&mut r_node.key, min_right.key);
                    self.update_height(&mut r_node, r_id);
                    r_key
                };

                // removing node might have caused an imbalance - balance the tree up to the root,
                // starting from the lowest affected key (max key from left subtree in this case)
                self.root = self.root.map(|root| {
                    self.check_balance(
                        root,
                        parent.as_ref().map(|p| &p.1.key).unwrap_or(&r_node.key),
                    )
                });
                Some(replaced_key)
            }
        }
    }
}

impl<K, V, H> TreeMap<K, V, H>
where
    K: BorshSerialize + Ord,
    V: BorshSerialize,
    H: ToKey,
{
    /// An iterator visiting all key-value pairs in arbitrary order.
    /// The iterator element type is `(&'a K, &'a V)`.
    pub fn iter(&self) -> Iter<K, V, H>
    where
        K: BorshDeserialize,
    {
        Iter::new(self)
    }

    /// An iterator visiting all key-value pairs in arbitrary order,
    /// with exclusive references to the values.
    /// The iterator element type is `(&'a K, &'a mut V)`.
    pub fn iter_mut(&mut self) -> IterMut<K, V, H>
    where
        K: BorshDeserialize,
    {
        IterMut::new(self)
    }

    /// An iterator visiting all keys in arbitrary order.
    /// The iterator element type is `&'a K`.
    pub fn keys(&self) -> Keys<K>
    where
        K: BorshDeserialize,
    {
        Keys::new(&self.tree)
    }

    /// An iterator visiting all values in arbitrary order.
    /// The iterator element type is `&'a V`.
    pub fn values(&self) -> Values<K, V, H>
    where
        K: BorshDeserialize,
    {
        Values::new(self)
    }

    /// A mutable iterator visiting all values in arbitrary order.
    /// The iterator element type is `&'a mut V`.
    pub fn values_mut(&mut self) -> ValuesMut<K, V, H>
    where
        K: BorshDeserialize,
    {
        ValuesMut::new(self)
    }

    /// Constructs a double-ended iterator over a sub-range of elements in the map.
    /// The simplest way is to use the range syntax `min..max`, thus `range(min..max)` will
    /// yield elements from min (inclusive) to max (exclusive).
    /// The range may also be entered as `(Bound<T>, Bound<T>)`, so for example
    /// `range((Excluded(4), Included(10)))` will yield a left-exclusive, right-inclusive
    /// range from 4 to 10.
    ///
    /// # Panics
    ///
    /// Panics if range `start > end`.
    /// Panics if range `start == end` and both bounds are `Excluded`.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use near_sdk::store::TreeMap;
    /// use std::ops::Bound::Included;
    ///
    /// let mut map = TreeMap::new(b"t");
    /// map.insert(3, "a".to_string());
    /// map.insert(5, "b".to_string());
    /// map.insert(8, "c".to_string());
    /// for (key, value) in map.range((Included(&4), Included(&8))) {
    ///     println!("{}: {}", key, value);
    /// }
    /// assert_eq!(Some((&5, &"b".to_string())), map.range(4..).next());
    /// ```
    pub fn range<'a, R: 'a, Q: 'a>(&'a self, range: R) -> Range<'a, K, V, H>
    where
        K: BorshDeserialize + Borrow<Q>,
        Q: ?Sized + Ord,
        R: RangeBounds<Q>,
    {
        Range::new(self, (range.start_bound(), range.end_bound()))
    }

    /// Constructs a mutable double-ended iterator over a sub-range of elements in the map.
    /// The simplest way is to use the range syntax `min..max`, thus `range(min..max)` will
    /// yield elements from min (inclusive) to max (exclusive).
    /// The range may also be entered as `(Bound<T>, Bound<T>)`, so for example
    /// `range((Excluded(4), Included(10)))` will yield a left-exclusive, right-inclusive
    /// range from 4 to 10.
    ///
    /// # Panics
    ///
    /// Panics if range `start > end`.
    /// Panics if range `start == end` and both bounds are `Excluded`.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use near_sdk::store::TreeMap;
    ///
    /// let mut map: TreeMap<i32, i32> = TreeMap::new(b"t");
    /// map.extend([4, 6, 8, 11]
    ///     .iter()
    ///     .map(|&s| (s, 0)));
    /// for (_, balance) in map.range_mut(6..10) {
    ///     *balance += 100;
    /// }
    /// for (id, balance) in &map {
    ///     println!("{} => {}", id, balance);
    /// }
    /// ```
    pub fn range_mut<R, Q>(&mut self, range: R) -> RangeMut<'_, K, V, H>
    where
        K: BorshDeserialize + Borrow<Q>,
        Q: ?Sized + Ord,
        R: RangeBounds<Q>,
    {
        RangeMut::new(self, (range.start_bound(), range.end_bound()))
    }
}

impl<K, V, H> TreeMap<K, V, H>
where
    K: BorshSerialize + Ord,
    V: BorshSerialize + BorshDeserialize,
    H: ToKey,
{
    /// Removes a key from the map, returning the stored key and value if the
    /// key was previously in the map.
    ///
    /// The key may be any borrowed form of the map's key type, but
    /// [`BorshSerialize`] and [`ToOwned<Owned = K>`](ToOwned) on the borrowed form *must* match
    /// those for the key type.
    ///
    /// # Examples
    ///
    /// ```
    /// use near_sdk::store::TreeMap;
    ///
    /// let mut map = TreeMap::new(b"m");
    /// map.insert(1, "a".to_string());
    /// assert_eq!(map.remove(&1), Some("a".to_string()));
    /// assert_eq!(map.remove(&1), None);
    /// ```
    pub fn remove_entry<Q: ?Sized>(&mut self, key: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q> + BorshDeserialize + Clone,
        Q: BorshSerialize + ToOwned<Owned = K> + Eq + PartialOrd,
    {
        self.values.remove(key).map(|removed_value| {
            let removed = self.tree.do_remove(key);
            (expect(removed), removed_value)
        })
    }

    /// Gets the given key's corresponding entry in the map for in-place manipulation.
    /// ```
    /// use near_sdk::store::TreeMap;
    ///
    /// let mut count = TreeMap::new(b"m");
    ///
    /// for ch in [7, 2, 4, 7, 4, 1, 7] {
    ///     let counter = count.entry(ch).or_insert(0);
    ///     *counter += 1;
    /// }
    ///
    /// assert_eq!(count[&4], 2);
    /// assert_eq!(count[&7], 3);
    /// assert_eq!(count[&1], 1);
    /// assert_eq!(count.get(&8), None);
    /// ```
    pub fn entry(&mut self, key: K) -> Entry<K, V>
    where
        K: Clone,
    {
        Entry::new(self.values.entry(key), &mut self.tree)
    }
}

impl<K, V, H> TreeMap<K, V, H>
where
    K: BorshSerialize + Ord,
    V: BorshSerialize,
    H: ToKey,
{
    /// Flushes the intermediate values of the map before this is called when the structure is
    /// [`Drop`]ed. This will write all modified values to storage but keep all cached values
    /// in memory.
    pub fn flush(&mut self) {
        self.values.flush();
        self.tree.nodes.flush();
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_env::setup_free;
    use crate::test_utils::{next_trie_id, test_env};

    use arbitrary::{Arbitrary, Unstructured};
    use quickcheck::QuickCheck;
    use rand::RngCore;
    use rand::SeedableRng;
    use std::collections::BTreeMap;
    use std::collections::HashSet;
    use std::ops::Bound;

    /// Return height of the tree - number of nodes on the longest path starting from the root node.
    fn height<K, V, H>(tree: &TreeMap<K, V, H>) -> u32
    where
        K: Ord + Clone + BorshSerialize + BorshDeserialize,
        V: BorshSerialize + BorshDeserialize,
        H: ToKey,
    {
        tree.tree.root.and_then(|root| tree.tree.node(root)).map(|n| n.ht).unwrap_or_default()
    }

    fn random(n: u32) -> Vec<u32> {
        let mut rng = rand::thread_rng();
        let mut vec = Vec::with_capacity(n as usize);
        (0..n).for_each(|_| {
            vec.push(rng.next_u32() % 1000);
        });
        vec
    }

    fn log2(x: f64) -> f64 {
        std::primitive::f64::log(x, 2.0f64)
    }

    fn max_tree_height(n: u32) -> u32 {
        // h <= C * log2(n + D) + B
        // where:
        // C =~ 1.440, D =~ 1.065, B =~ 0.328
        // (source: https://en.wikipedia.org/wiki/AVL_tree)
        const B: f64 = -0.328;
        const C: f64 = 1.440;
        const D: f64 = 1.065;

        let h = C * log2(n as f64 + D) + B;
        h.ceil() as u32
    }

    #[test]
    fn test_empty() {
        let map: TreeMap<u8, u8> = TreeMap::new(b't');
        assert_eq!(map.len(), 0);
        assert_eq!(height(&map), 0);
        assert_eq!(map.get(&42), None);
        assert!(!map.contains_key(&42));
        assert_eq!(map.tree.lower(&42), None);
        assert_eq!(map.tree.higher(&42), None);
    }

    #[test]
    fn test_insert_3_rotate_l_l() {
        let mut map: TreeMap<i32, i32> = TreeMap::new(next_trie_id());
        assert_eq!(height(&map), 0);

        map.insert(3, 3);
        assert_eq!(height(&map), 1);

        map.insert(2, 2);
        assert_eq!(height(&map), 2);

        map.insert(1, 1);
        assert_eq!(height(&map), 2);

        let root = map.tree.root.unwrap();
        assert_eq!(root, FreeListIndex(1));
        assert_eq!(map.tree.node(root).map(|n| n.key), Some(2));

        map.clear();
    }

    #[test]
    fn test_insert_3_rotate_r_r() {
        let mut map: TreeMap<i32, i32> = TreeMap::new(next_trie_id());
        assert_eq!(height(&map), 0);

        map.insert(1, 1);
        assert_eq!(height(&map), 1);

        map.insert(2, 2);
        assert_eq!(height(&map), 2);

        map.insert(3, 3);

        let root = map.tree.root.unwrap();
        assert_eq!(root, FreeListIndex(1));
        assert_eq!(map.tree.node(root).map(|n| n.key), Some(2));
        assert_eq!(height(&map), 2);

        map.clear();
    }

    #[test]
    fn test_insert_lookup_n_asc() {
        let mut map: TreeMap<i32, i32> = TreeMap::new(next_trie_id());

        let n: u32 = 30;
        let cases = (0..2 * (n as i32)).collect::<Vec<i32>>();

        let mut counter = 0;
        for k in cases.iter().copied() {
            if k % 2 == 0 {
                counter += 1;
                map.insert(k, counter);
            }
        }

        counter = 0;
        for k in &cases {
            if *k % 2 == 0 {
                counter += 1;
                assert_eq!(map.get(k), Some(&counter));
            } else {
                assert_eq!(map.get(k), None);
            }
        }

        assert!(height(&map) <= max_tree_height(n));
        map.clear();
    }

    #[test]
    pub fn test_insert_one() {
        let mut map = TreeMap::new(b"m");
        assert_eq!(None, map.insert(1, 2));
        assert_eq!(2, map.insert(1, 3).unwrap());
    }

    #[test]
    fn test_insert_lookup_n_desc() {
        let mut map: TreeMap<i32, i32> = TreeMap::new(next_trie_id());

        let n: u32 = 30;
        let cases = (0..2 * (n as i32)).rev().collect::<Vec<i32>>();

        let mut counter = 0;
        for k in cases.iter().copied() {
            if k % 2 == 0 {
                counter += 1;
                map.insert(k, counter);
            }
        }

        counter = 0;
        for k in &cases {
            if *k % 2 == 0 {
                counter += 1;
                assert_eq!(map.get(k), Some(&counter));
            } else {
                assert_eq!(map.get(k), None);
            }
        }

        assert!(height(&map) <= max_tree_height(n));
        map.clear();
    }

    #[test]
    fn insert_n_random() {
        test_env::setup_free();

        for k in 1..10 {
            // tree size is 2^k
            let mut map: TreeMap<u32, u32> = TreeMap::new(next_trie_id());

            let n = 1 << k;
            let input: Vec<u32> = random(n);

            for x in input.iter().copied() {
                map.insert(x, 42);
            }

            for x in &input {
                assert_eq!(map.get(x), Some(&42));
            }

            assert!(height(&map) <= max_tree_height(n));
            map.clear();
        }
    }

    // #[test]
    // fn test_min() {
    //     let n: u32 = 30;
    //     let vec = random(n);

    //     let mut map: TreeMap<u32, u32> = TreeMap::new(b't');
    //     for x in vec.iter().rev().copied() {
    //         map.insert(x, 1);
    //     }

    //     assert_eq!(map.min().unwrap(), *vec.iter().min().unwrap());
    //     map.clear();
    // }

    #[test]
    fn test_max() {
        let n: u32 = 30;
        let vec = random(n);

        let mut map: TreeMap<u32, u32> = TreeMap::new(b't');
        for x in vec.iter().rev().copied() {
            map.insert(x, 1);
        }

        let tree_max = map.tree.max_at(map.tree.root.unwrap()).map(|((_, n), _)| &n.key);

        assert_eq!(tree_max.unwrap(), vec.iter().max().unwrap());
        map.clear();
    }

    #[test]
    fn test_lower() {
        let mut map: TreeMap<u32, u32> = TreeMap::new(next_trie_id());
        let vec = [10, 20, 30, 40, 50];

        for x in vec.into_iter() {
            map.insert(x, 1);
        }

        assert_eq!(map.tree.lower(&5), None);
        assert_eq!(map.tree.lower(&10), None);
        assert_eq!(map.tree.lower(&11), Some(&10));
        assert_eq!(map.tree.lower(&20), Some(&10));
        assert_eq!(map.tree.lower(&49), Some(&40));
        assert_eq!(map.tree.lower(&50), Some(&40));
        assert_eq!(map.tree.lower(&51), Some(&50));

        map.clear();
    }

    #[test]
    fn test_higher() {
        let mut map: TreeMap<u32, u32> = TreeMap::new(next_trie_id());
        let vec = [10, 20, 30, 40, 50];

        for x in vec.into_iter() {
            map.insert(x, 1);
        }

        assert_eq!(map.tree.higher(&5), Some(&10));
        assert_eq!(map.tree.higher(&10), Some(&20));
        assert_eq!(map.tree.higher(&11), Some(&20));
        assert_eq!(map.tree.higher(&20), Some(&30));
        assert_eq!(map.tree.higher(&49), Some(&50));
        assert_eq!(map.tree.higher(&50), None);
        assert_eq!(map.tree.higher(&51), None);

        map.clear();
    }

    #[test]
    fn test_floor_key() {
        let mut map: TreeMap<u32, u32> = TreeMap::new(next_trie_id());
        let vec = [10, 20, 30, 40, 50];

        for x in vec.into_iter() {
            map.insert(x, 1);
        }

        assert_eq!(map.tree.floor_key(&5), None);
        assert_eq!(map.tree.floor_key(&10), Some(&10));
        assert_eq!(map.tree.floor_key(&11), Some(&10));
        assert_eq!(map.tree.floor_key(&20), Some(&20));
        assert_eq!(map.tree.floor_key(&49), Some(&40));
        assert_eq!(map.tree.floor_key(&50), Some(&50));
        assert_eq!(map.tree.floor_key(&51), Some(&50));

        map.clear();
    }

    #[test]
    fn test_ceil_key() {
        let mut map: TreeMap<u32, u32> = TreeMap::new(next_trie_id());
        let vec = [10, 20, 30, 40, 50];

        for x in vec.into_iter() {
            map.insert(x, 1);
        }

        assert_eq!(map.tree.ceil_key(&5), Some(&10));
        assert_eq!(map.tree.ceil_key(&10), Some(&10));
        assert_eq!(map.tree.ceil_key(&11), Some(&20));
        assert_eq!(map.tree.ceil_key(&20), Some(&20));
        assert_eq!(map.tree.ceil_key(&49), Some(&50));
        assert_eq!(map.tree.ceil_key(&50), Some(&50));
        assert_eq!(map.tree.ceil_key(&51), None);

        map.clear();
    }

    #[test]
    fn test_remove_1() {
        let mut map: TreeMap<u32, u32> = TreeMap::new(next_trie_id());
        map.insert(1, 1);
        assert_eq!(map.get(&1), Some(&1));
        map.remove(&1);
        assert_eq!(map.get(&1), None);
        assert_eq!(map.tree.nodes.len(), 0);
        map.clear();
    }

    #[test]
    fn test_remove_3() {
        let map: TreeMap<u32, u32> = avl(&[(0, 0)], &[0, 0, 1]);

        assert!(map.is_empty());
    }

    #[test]
    fn test_remove_3_desc() {
        let vec = [3, 2, 1];
        let mut map: TreeMap<u32, u32> = TreeMap::new(next_trie_id());

        for x in &vec {
            assert_eq!(map.get(x), None);
            map.insert(*x, 1);
            assert_eq!(map.get(x), Some(&1));
        }

        for x in &vec {
            assert_eq!(map.get(x), Some(&1));
            map.remove(x);
            assert_eq!(map.get(x), None);
        }
        map.clear();
    }

    #[test]
    fn test_remove_3_asc() {
        let vec = [1, 2, 3];
        let mut map: TreeMap<u32, u32> = TreeMap::new(next_trie_id());

        for x in &vec {
            assert_eq!(map.get(x), None);
            map.insert(*x, 1);
            assert_eq!(map.get(x), Some(&1));
        }
        assert_eq!(map.tree.nodes.get(FreeListIndex(0)).unwrap().key, 1);

        for x in &vec {
            assert_eq!(map.get(x), Some(&1));
            map.remove(x);
            assert_eq!(map.get(x), None);
        }
        map.clear();
    }

    #[test]
    fn test_remove_7_regression_1() {
        let vec =
            [2104297040, 552624607, 4269683389, 3382615941, 155419892, 4102023417, 1795725075];
        let mut map: TreeMap<u32, u32> = TreeMap::new(next_trie_id());

        for x in &vec {
            assert_eq!(map.get(x), None);
            map.insert(*x, 1);
            assert_eq!(map.get(x), Some(&1));
        }

        assert!(is_balanced(&map, map.tree.root.unwrap()));

        for x in &vec {
            assert_eq!(map.get(x), Some(&1));
            map.remove(x);
            assert_eq!(map.get(x), None);
        }
        map.clear();
    }

    #[test]
    fn test_remove_7_regression_2() {
        let vec = [700623085, 87488544, 1500140781, 1111706290, 3187278102, 4042663151, 3731533080];
        let mut map: TreeMap<u32, u32> = TreeMap::new(next_trie_id());

        for x in &vec {
            assert_eq!(map.get(x), None);
            map.insert(*x, 1);
            assert_eq!(map.get(x), Some(&1));
        }

        for x in &vec {
            assert_eq!(map.get(x), Some(&1));
            map.remove(x);
            assert_eq!(map.get(x), None);
        }
        map.clear();
    }

    #[test]
    fn test_remove_9_regression() {
        let vec = [
            1186903464, 506371929, 1738679820, 1883936615, 1815331350, 1512669683, 3581743264,
            1396738166, 1902061760,
        ];
        let mut map: TreeMap<u32, u32> = TreeMap::new(next_trie_id());

        for x in &vec {
            assert_eq!(map.get(x), None);
            map.insert(*x, 1);
            assert_eq!(map.get(x), Some(&1));
        }

        for x in &vec {
            assert_eq!(map.get(x), Some(&1));
            map.remove(x);
            assert_eq!(map.get(x), None);
        }
        map.clear();
    }

    #[test]
    fn test_remove_20_regression_1() {
        let vec = [
            552517392, 3638992158, 1015727752, 2500937532, 638716734, 586360620, 2476692174,
            1425948996, 3608478547, 757735878, 2709959928, 2092169539, 3620770200, 783020918,
            1986928932, 200210441, 1972255302, 533239929, 497054557, 2137924638,
        ];
        let mut map: TreeMap<u32, u32> = TreeMap::new(next_trie_id());

        for x in &vec {
            assert_eq!(map.get(x), None);
            map.insert(*x, 1);
            assert_eq!(map.get(x), Some(&1));
        }

        for x in &vec {
            assert_eq!(map.get(x), Some(&1));
            map.remove(x);
            assert_eq!(map.get(x), None);
        }
        map.clear();
    }

    #[test]
    fn test_remove_7_regression() {
        let vec = [280, 606, 163, 857, 436, 508, 44, 801];

        let mut map: TreeMap<u32, u32> = TreeMap::new(next_trie_id());

        for x in &vec {
            assert_eq!(map.get(x), None);
            map.insert(*x, 1);
            assert_eq!(map.get(x), Some(&1));
        }

        for x in &vec {
            assert_eq!(map.get(x), Some(&1));
            map.remove(x);
            assert_eq!(map.get(x), None);
        }

        assert_eq!(map.len(), 0, "map.len() > 0");
        assert_eq!(map.tree.nodes.len(), 0, "map.tree is not empty");
        map.clear();
    }

    #[test]
    fn test_insert_8_remove_4_regression() {
        let insert = [882, 398, 161, 76];
        let remove = [242, 687, 860, 811];

        let mut map: TreeMap<u32, u32> = TreeMap::new(next_trie_id());

        for (i, (k1, k2)) in insert.iter().zip(remove.iter()).enumerate() {
            let v = i as u32;
            map.insert(*k1, v);
            map.insert(*k2, v);
        }

        for k in remove.iter() {
            map.remove(k);
        }

        assert_eq!(map.len(), insert.len() as u32);

        for (i, k) in (0..).zip(insert.iter()) {
            assert_eq!(map.get(k), Some(&i));
        }
    }

    #[test]
    fn test_remove_n() {
        let n: u32 = 20;
        let vec = random(n);

        let mut set: HashSet<u32> = HashSet::new();
        let mut map: TreeMap<u32, u32> = TreeMap::new(next_trie_id());
        for x in &vec {
            map.insert(*x, 1);
            set.insert(*x);
        }

        assert_eq!(map.len(), set.len() as u32);

        for x in &set {
            assert_eq!(map.get(x), Some(&1));
            map.remove(x);
            assert_eq!(map.get(x), None);
        }

        assert_eq!(map.len(), 0, "map.len() > 0");
        assert_eq!(map.tree.nodes.len(), 0, "map.tree is not empty");
        map.clear();
    }

    #[test]
    fn test_remove_root_3() {
        let mut map: TreeMap<u32, u32> = TreeMap::new(next_trie_id());
        map.insert(2, 1);
        map.insert(3, 1);
        map.insert(1, 1);
        map.insert(4, 1);

        map.remove(&2);

        assert_eq!(map.get(&1), Some(&1));
        assert_eq!(map.get(&2), None);
        assert_eq!(map.get(&3), Some(&1));
        assert_eq!(map.get(&4), Some(&1));
        map.clear();
    }

    #[test]
    fn test_insert_2_remove_2_regression() {
        let ins = [11760225, 611327897];
        let rem = [2982517385, 1833990072];

        let mut map: TreeMap<u32, u32> = TreeMap::new(next_trie_id());
        map.insert(ins[0], 1);
        map.insert(ins[1], 1);

        map.remove(&rem[0]);
        map.remove(&rem[1]);

        let h = height(&map);
        let h_max = max_tree_height(map.len());
        assert!(h <= h_max, "h={} h_max={}", h, h_max);
        map.clear();
    }

    #[test]
    fn test_insert_n_duplicates() {
        let mut map: TreeMap<u32, u32> = TreeMap::new(next_trie_id());

        for x in 0..30 {
            map.insert(x, x);
            map.insert(42, x);
        }

        assert_eq!(map.get(&42), Some(&29));
        assert_eq!(map.len(), 31);
        assert_eq!(map.tree.nodes.len(), 31);

        map.clear();
    }

    #[test]
    fn test_insert_2n_remove_n_random() {
        for k in 1..4 {
            let mut map: TreeMap<u32, u32> = TreeMap::new(next_trie_id());
            let mut set: HashSet<u32> = HashSet::new();

            let n = 1 << k;
            let ins: Vec<u32> = random(n);
            let rem: Vec<u32> = random(n);

            for x in &ins {
                set.insert(*x);
                map.insert(*x, 42);
            }

            for x in &rem {
                set.insert(*x);
                map.insert(*x, 42);
            }

            for x in &rem {
                set.remove(x);
                map.remove(x);
            }

            assert_eq!(map.len(), set.len() as u32);

            let h = height(&map);
            let h_max = max_tree_height(n);
            assert!(h <= h_max, "[n={}] tree is too high: {} (max is {}).", n, h, h_max);

            map.clear();
        }
    }

    #[test]
    fn test_remove_empty() {
        let mut map: TreeMap<u32, u32> = TreeMap::new(next_trie_id());
        assert_eq!(map.remove(&1), None);
    }

    #[test]
    fn test_iter() {
        let mut map: TreeMap<u32, u32> = TreeMap::new(next_trie_id());
        map.insert(1, 41);
        map.insert(2, 42);
        map.insert(3, 43);

        assert_eq!(map.iter().collect::<Vec<_>>(), vec![(&1, &41), (&2, &42), (&3, &43)]);

        // Test custom iterator impls
        assert_eq!(map.iter().nth(1), Some((&2, &42)));
        assert_eq!(map.iter().count(), 3);
        assert_eq!(map.iter().last(), Some((&3, &43)));
        map.clear();
    }

    #[test]
    fn test_iter_empty() {
        let map: TreeMap<u32, u32> = TreeMap::new(next_trie_id());
        assert_eq!(map.iter().count(), 0);
    }

    #[test]
    fn test_iter_rev() {
        let mut map: TreeMap<u32, u32> = TreeMap::new(next_trie_id());
        map.insert(1, 41);
        map.insert(2, 42);
        map.insert(3, 43);

        assert_eq!(
            map.iter().rev().collect::<Vec<(&u32, &u32)>>(),
            vec![(&3, &43), (&2, &42), (&1, &41)]
        );

        // Test custom iterator impls
        assert_eq!(map.iter().rev().nth(1), Some((&2, &42)));
        assert_eq!(map.iter().rev().count(), 3);
        assert_eq!(map.iter().rev().last(), Some((&1, &41)));
        map.clear();
    }

    #[test]
    fn test_iter_rev_empty() {
        let map: TreeMap<u32, u32> = TreeMap::new(next_trie_id());
        assert_eq!(map.iter().rev().count(), 0);
    }

    #[test]
    fn test_iter_from() {
        let mut map: TreeMap<u32, u32> = TreeMap::new(next_trie_id());

        let one = [10, 20, 30, 40, 50];
        let two = [45, 35, 25, 15, 5];

        for x in &one {
            map.insert(*x, 42);
        }

        for x in &two {
            map.insert(*x, 42);
        }

        assert_eq!(
            map.range(30..).map(|(&a, &b)| (a, b)).collect::<Vec<(u32, u32)>>(),
            vec![(30, 42), (35, 42), (40, 42), (45, 42), (50, 42)]
        );

        assert_eq!(
            map.range(31..).map(|(&a, &b)| (a, b)).collect::<Vec<(u32, u32)>>(),
            vec![(35, 42), (40, 42), (45, 42), (50, 42)]
        );

        // Test custom iterator impls
        assert_eq!(map.range(31..).nth(2), Some((&45, &42)));
        assert_eq!(map.range(31..).count(), 4);
        assert_eq!(map.range(31..).last(), Some((&50, &42)));

        map.clear();
    }

    #[test]
    fn test_iter_from_empty() {
        let map: TreeMap<u32, u32> = TreeMap::new(next_trie_id());
        assert_eq!(map.range(42..).count(), 0);
    }

    #[test]
    fn test_iter_rev_from() {
        let mut map: TreeMap<u32, u32> = TreeMap::new(next_trie_id());

        let one = [10, 20, 30, 40, 50];
        let two = [45, 35, 25, 15, 5];

        for x in &one {
            map.insert(*x, 42);
        }

        for x in &two {
            map.insert(*x, 42);
        }

        assert_eq!(
            map.range(..29).rev().map(|(&a, &b)| (a, b)).collect::<Vec<(u32, u32)>>(),
            vec![(25, 42), (20, 42), (15, 42), (10, 42), (5, 42)]
        );

        assert_eq!(
            map.range(..30).rev().map(|(&a, &b)| (a, b)).collect::<Vec<(u32, u32)>>(),
            vec![(25, 42), (20, 42), (15, 42), (10, 42), (5, 42)]
        );

        assert_eq!(
            map.range(..31).rev().map(|(&a, &b)| (a, b)).collect::<Vec<(u32, u32)>>(),
            vec![(30, 42), (25, 42), (20, 42), (15, 42), (10, 42), (5, 42)]
        );

        // Test custom iterator impls
        assert_eq!(map.range(..31).rev().nth(2), Some((&20, &42)));
        assert_eq!(map.range(..31).rev().count(), 6);
        assert_eq!(map.range(..31).rev().last(), Some((&5, &42)));

        map.clear();
    }

    #[test]
    fn test_range() {
        let mut map: TreeMap<u32, u32> = TreeMap::new(next_trie_id());

        let one = [10, 20, 30, 40, 50];
        let two = [45, 35, 25, 15, 5];

        for x in &one {
            map.insert(*x, 42);
        }

        for x in &two {
            map.insert(*x, 42);
        }

        assert_eq!(
            map.range((Bound::Included(20), Bound::Excluded(30)))
                .map(|(&a, &b)| (a, b))
                .collect::<Vec<(u32, u32)>>(),
            vec![(20, 42), (25, 42)]
        );

        assert_eq!(
            map.range((Bound::Excluded(10), Bound::Included(40)))
                .map(|(&a, &b)| (a, b))
                .collect::<Vec<(u32, u32)>>(),
            vec![(15, 42), (20, 42), (25, 42), (30, 42), (35, 42), (40, 42)]
        );

        assert_eq!(
            map.range((Bound::Included(20), Bound::Included(40)))
                .map(|(&a, &b)| (a, b))
                .collect::<Vec<(u32, u32)>>(),
            vec![(20, 42), (25, 42), (30, 42), (35, 42), (40, 42)]
        );

        assert_eq!(
            map.range((Bound::Excluded(20), Bound::Excluded(45)))
                .map(|(&a, &b)| (a, b))
                .collect::<Vec<(u32, u32)>>(),
            vec![(25, 42), (30, 42), (35, 42), (40, 42)]
        );

        assert_eq!(
            map.range((Bound::Excluded(25), Bound::Excluded(30)))
                .map(|(&a, &b)| (a, b))
                .collect::<Vec<(u32, u32)>>(),
            vec![]
        );

        assert_eq!(
            map.range((Bound::Included(25), Bound::Included(25)))
                .map(|(&a, &b)| (a, b))
                .collect::<Vec<(u32, u32)>>(),
            vec![(25, 42)]
        );

        assert_eq!(
            map.range((Bound::Excluded(25), Bound::Included(25)))
                .map(|(&a, &b)| (a, b))
                .collect::<Vec<(u32, u32)>>(),
            vec![]
        ); // the range makes no sense, but `BTreeMap` does not panic in this case

        // Test custom iterator impls
        assert_eq!(map.range((Bound::Excluded(20), Bound::Excluded(45))).nth(2), Some((&35, &42)));
        assert_eq!(map.range((Bound::Excluded(20), Bound::Excluded(45))).count(), 4);
        assert_eq!(map.range((Bound::Excluded(20), Bound::Excluded(45))).last(), Some((&40, &42)));

        map.clear();
    }

    #[test]
    fn test_iter_rev_from_empty() {
        let map: TreeMap<u32, u32> = TreeMap::new(next_trie_id());
        assert_eq!(map.range(..=42).rev().count(), 0);
    }

    #[test]
    fn test_balance_regression_1() {
        let insert = [(2, 0), (3, 0), (4, 0)];
        let remove = [0, 0, 0, 1];

        let map = avl(&insert, &remove);
        assert!(is_balanced(&map, map.tree.root.unwrap()));
    }

    #[test]
    fn test_balance_regression_2() {
        let insert = [(1, 0), (2, 0), (0, 0), (3, 0), (5, 0), (6, 0)];
        let remove = [0, 0, 0, 3, 5, 6, 7, 4];

        let map = avl(&insert, &remove);
        assert!(is_balanced(&map, map.tree.root.unwrap()));
    }

    //
    // Property-based tests of AVL-based TreeMap against std::collections::BTreeMap
    //

    fn avl<K, V>(insert: &[(K, V)], remove: &[K]) -> TreeMap<K, V, Sha256>
    where
        K: Ord + Clone + BorshSerialize + BorshDeserialize,
        V: Default + BorshSerialize + BorshDeserialize + Clone,
    {
        test_env::setup_free();
        let mut map: TreeMap<K, V, _> = TreeMap::new(next_trie_id());
        for k in remove {
            map.insert(k.clone(), Default::default());
        }
        let n = insert.len().max(remove.len());
        for i in 0..n {
            if i < remove.len() {
                map.remove(&remove[i]);
            }
            if i < insert.len() {
                let (k, v) = &insert[i];
                map.insert(k.clone(), v.clone());
            }
        }
        map
    }

    fn rb<K, V>(insert: &[(K, V)], remove: &[K]) -> BTreeMap<K, V>
    where
        K: Ord + Clone + BorshSerialize + BorshDeserialize,
        V: Clone + Default + BorshSerialize + BorshDeserialize,
    {
        let mut map: BTreeMap<K, V> = BTreeMap::default();
        for k in remove {
            map.insert(k.clone(), Default::default());
        }
        let n = insert.len().max(remove.len());
        for i in 0..n {
            if i < remove.len() {
                map.remove(&remove[i]);
            }
            if i < insert.len() {
                let (k, v) = &insert[i];
                map.insert(k.clone(), v.clone());
            }
        }
        map
    }

    #[test]
    fn prop_avl_vs_rb() {
        fn prop(insert: Vec<(u32, u32)>, remove: Vec<u32>) -> bool {
            let a = avl(&insert, &remove);
            let b = rb(&insert, &remove);
            let v1: Vec<(&u32, &u32)> = a.iter().collect();
            let v2: Vec<(&u32, &u32)> = b.iter().collect();
            v1 == v2
        }

        QuickCheck::new()
            .tests(300)
            .quickcheck(prop as fn(std::vec::Vec<(u32, u32)>, std::vec::Vec<u32>) -> bool);
    }

    #[test]
    fn insert_delete_insert() {
        let mut map = TreeMap::new(b"t");
        map.insert(0, 0);
        assert_eq!(map.remove(&0), Some(0));
        map.insert(0, 0);
        assert!(is_balanced(&map, map.tree.root.unwrap()));
    }

    fn is_balanced<K, V, H>(map: &TreeMap<K, V, H>, root: FreeListIndex) -> bool
    where
        K: Ord + Clone + BorshSerialize + BorshDeserialize,
        V: BorshSerialize + BorshDeserialize,
        H: ToKey,
    {
        let node = map.tree.node(root).unwrap();
        let balance = map.tree.get_balance(node);

        (-1..=1).contains(&balance)
            && node.lft.map(|id| is_balanced(map, id)).unwrap_or(true)
            && node.rgt.map(|id| is_balanced(map, id)).unwrap_or(true)
    }

    #[test]
    fn prop_avl_balance() {
        test_env::setup_free();

        fn prop(insert: Vec<(u32, u32)>, remove: Vec<u32>) -> bool {
            let map = avl(&insert, &remove);
            map.is_empty() || is_balanced(&map, map.tree.root.unwrap())
        }

        QuickCheck::new()
            .tests(300)
            .quickcheck(prop as fn(std::vec::Vec<(u32, u32)>, std::vec::Vec<u32>) -> bool);
    }

    #[test]
    fn prop_avl_height() {
        test_env::setup_free();

        fn prop(insert: Vec<(u32, u32)>, remove: Vec<u32>) -> bool {
            let map = avl(&insert, &remove);
            height(&map) <= max_tree_height(map.len())
        }

        QuickCheck::new()
            .tests(300)
            .quickcheck(prop as fn(std::vec::Vec<(u32, u32)>, std::vec::Vec<u32>) -> bool);
    }

    fn range_prop(
        insert: Vec<(u32, u32)>,
        remove: Vec<u32>,
        range: (Bound<u32>, Bound<u32>),
    ) -> bool {
        let a = avl(&insert, &remove);
        let b = rb(&insert, &remove);
        let v1: Vec<(&u32, &u32)> = a.range(range).collect();
        let v2: Vec<(&u32, &u32)> = b.range(range).collect();
        v1 == v2
    }

    type Prop = fn(std::vec::Vec<(u32, u32)>, std::vec::Vec<u32>, u32, u32) -> bool;

    #[test]
    fn prop_avl_vs_rb_range_incl_incl() {
        fn prop(insert: Vec<(u32, u32)>, remove: Vec<u32>, r1: u32, r2: u32) -> bool {
            let range = (Bound::Included(r1.min(r2)), Bound::Included(r1.max(r2)));
            range_prop(insert, remove, range)
        }

        QuickCheck::new().tests(300).quickcheck(prop as Prop);
    }

    #[test]
    fn prop_avl_vs_rb_range_incl_excl() {
        fn prop(insert: Vec<(u32, u32)>, remove: Vec<u32>, r1: u32, r2: u32) -> bool {
            let range = (Bound::Included(r1.min(r2)), Bound::Excluded(r1.max(r2)));
            range_prop(insert, remove, range)
        }

        QuickCheck::new().tests(300).quickcheck(prop as Prop);
    }

    #[test]
    fn prop_avl_vs_rb_range_excl_incl() {
        fn prop(insert: Vec<(u32, u32)>, remove: Vec<u32>, r1: u32, r2: u32) -> bool {
            let range = (Bound::Excluded(r1.min(r2)), Bound::Included(r1.max(r2)));
            range_prop(insert, remove, range)
        }

        QuickCheck::new().tests(300).quickcheck(prop as Prop);
    }

    #[test]
    fn prop_avl_vs_rb_range_excl_excl() {
        fn prop(insert: Vec<(u32, u32)>, remove: Vec<u32>, r1: u32, r2: u32) -> bool {
            // (Excluded(x), Excluded(x)) is invalid range, checking against it makes no sense
            r1 == r2 || {
                let range = (Bound::Excluded(r1.min(r2)), Bound::Excluded(r1.max(r2)));
                range_prop(insert, remove, range)
            }
        }

        QuickCheck::new().tests(300).quickcheck(prop as Prop);
    }

    #[test]
    fn entry_api() {
        let mut map = TreeMap::new(b"b");
        {
            let test_entry = map.entry("test".to_string());
            assert_eq!(test_entry.key(), "test");
            let entry_ref = test_entry.or_insert(8u8);
            *entry_ref += 1;
        }
        assert_eq!(map["test"], 9);

        // Try getting entry of filled value
        let value = map.entry("test".to_string()).and_modify(|v| *v += 3).or_default();
        assert_eq!(*value, 12);
    }

    #[test]
    fn map_iterator() {
        let mut map = TreeMap::new(b"b");

        map.insert(0u8, 0u8);
        map.insert(1, 1);
        map.insert(2, 2);
        map.insert(3, 3);
        map.remove(&1);
        let iter = map.iter();
        assert_eq!(iter.len(), 3);
        assert_eq!(iter.collect::<Vec<_>>(), [(&0, &0), (&2, &2), (&3, &3)]);

        let iter = map.iter_mut().rev();
        assert_eq!(iter.collect::<Vec<_>>(), [(&3, &mut 3), (&2, &mut 2), (&0, &mut 0)]);

        let mut iter = map.iter();
        assert_eq!(iter.nth(2), Some((&3, &3)));
        // Check fused iterator assumption that each following one will be None
        assert_eq!(iter.next(), None);

        // Double all values
        map.values_mut().for_each(|v| {
            *v *= 2;
        });
        assert_eq!(map.values().collect::<Vec<_>>(), [&0, &4, &6]);

        // Collect all keys
        assert_eq!(map.keys().collect::<Vec<_>>(), [&0, &2, &3]);
    }

    #[derive(Arbitrary, Debug)]
    enum Op {
        Insert(u8, u8),
        Remove(u8),
        Flush,
        Restore,
        Get(u8),
        EntryInsert(u8, u8),
        EntryRemove(u8),
    }

    #[test]
    fn arbitrary() {
        setup_free();

        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(0);
        let mut buf = vec![0; 4096];
        for _ in 0..256 {
            // Clear storage in-between runs
            crate::mock::with_mocked_blockchain(|b| b.take_storage());
            rng.fill_bytes(&mut buf);

            let mut um = TreeMap::new(b"l");
            let mut hm = BTreeMap::new();
            let u = Unstructured::new(&buf);
            if let Ok(ops) = Vec::<Op>::arbitrary_take_rest(u) {
                for op in ops {
                    match op {
                        Op::Insert(k, v) => {
                            let r1 = um.insert(k, v);
                            let r2 = hm.insert(k, v);
                            assert_eq!(r1, r2)
                        }
                        Op::Remove(k) => {
                            let r1 = um.remove(&k);
                            let r2 = hm.remove(&k);
                            assert_eq!(r1, r2)
                        }
                        Op::Flush => {
                            um.flush();
                        }
                        Op::Restore => {
                            let serialized = borsh::to_vec(&um).unwrap();
                            um = TreeMap::deserialize(&mut serialized.as_slice()).unwrap();
                        }
                        Op::Get(k) => {
                            let r1 = um.get(&k);
                            let r2 = hm.get(&k);
                            assert_eq!(r1, r2)
                        }
                        Op::EntryInsert(k, v) => {
                            let r1 = um.entry(k).or_insert(v);
                            let r2 = hm.entry(k).or_insert(v);
                            assert_eq!(r1, r2)
                        }
                        Op::EntryRemove(k) => match (um.entry(k), hm.entry(k)) {
                            (
                                Entry::Occupied(o1),
                                std::collections::btree_map::Entry::Occupied(o2),
                            ) => {
                                let r1 = o1.remove();
                                let r2 = o2.remove();
                                assert_eq!(r1, r2)
                            }
                            (Entry::Vacant(_), std::collections::btree_map::Entry::Vacant(_)) => {}
                            _ => panic!("inconsistent entry states"),
                        },
                    }
                }
            }
        }
    }

    #[test]
    fn issue993() {
        fn swap_set<H>(map: &mut TreeMap<(), (), H>)
        where
            H: ToKey,
        {
            match map.entry(()) {
                Entry::Occupied(o) => {
                    o.remove();
                }
                Entry::Vacant(o) => {
                    o.insert(());
                }
            };
        }

        let mut map = TreeMap::new(b"m");
        swap_set(&mut map);
        assert_eq!(map.tree.root, Some(FreeListIndex(0)));
        swap_set(&mut map);
        assert_eq!(map.tree.root, None);
        // This line previously panicked because the entry was removed without updating the tree
        // root.
        swap_set(&mut map);
        assert_eq!(map.tree.root, Some(FreeListIndex(0)));
    }

    #[cfg(feature = "abi")]
    #[test]
    fn test_borsh_schema() {
        #[derive(
            borsh::BorshSerialize, borsh::BorshDeserialize, PartialEq, Eq, PartialOrd, Ord,
        )]
        struct NoSchemaStruct;

        assert_eq!(
            "TreeMap".to_string(),
            <TreeMap<NoSchemaStruct, NoSchemaStruct> as borsh::BorshSchema>::declaration()
        );
        let mut defs = Default::default();
        <TreeMap<NoSchemaStruct, NoSchemaStruct> as borsh::BorshSchema>::add_definitions_recursively(&mut defs);

        insta::assert_snapshot!(format!("{:#?}", defs));
    }
}
