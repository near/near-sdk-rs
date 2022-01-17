mod lazy;
pub use lazy::Lazy;

mod lazy_option;
pub use lazy_option::LazyOption;

pub mod vec;
pub use vec::Vector;

pub mod lookup_map;
pub use self::lookup_map::LookupMap;

mod lookup_set;
pub use self::lookup_set::LookupSet;

pub mod unordered_map;
pub use self::unordered_map::UnorderedMap;

pub mod unordered_set;
pub use self::unordered_set::UnorderedSet;

pub mod tree_map;
pub use self::tree_map::TreeMap;

mod index_map;
pub(crate) use self::index_map::IndexMap;

pub(crate) mod free_list;
pub(crate) use self::free_list::FreeList;

pub(crate) const ERR_INCONSISTENT_STATE: &str =
    "The collection is an inconsistent state. Did previous smart \
        contract execution terminate unexpectedly?";

// TODO don't need crate pub once moved
pub(crate) const ERR_NOT_EXIST: &str = "Key does not exist in map";
