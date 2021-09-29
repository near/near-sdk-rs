mod lazy;
pub use lazy::Lazy;

mod lazy_option;
pub use lazy_option::LazyOption;

pub mod vec;
pub use vec::Vector;

pub mod lookup_map;
pub use self::lookup_map::LookupMap;

mod index_map;
pub(crate) use self::index_map::IndexMap;

pub(crate) mod bucket;
#[allow(unused_imports)]
pub(crate) use self::bucket::Bucket;

const ERR_INCONSISTENT_STATE: &str = "The collection is an inconsistent state. Did previous smart \
										contract execution terminate unexpectedly?";
