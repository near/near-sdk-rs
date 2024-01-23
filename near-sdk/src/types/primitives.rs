#[cfg(all(not(target_arch = "wasm32"), feature = "unit-testing"))]
use near_primitives_core::hash::CryptoHash;

#[cfg(all(not(target_arch = "wasm32"), feature = "unit-testing"))]
pub use near_parameters::RuntimeFeesConfig;

//* Type aliases from near_primitives_core

/// Hash used by a struct implementing the Merkle tree.
#[cfg(all(not(target_arch = "wasm32"), feature = "unit-testing"))]
#[deprecated(since = "4.0.0", note = "Type has no connection with the SDK")]
pub type MerkleHash = CryptoHash;
/// Validator identifier in current group.
#[deprecated(since = "4.0.0", note = "Type has no connection with the SDK")]
pub type ValidatorId = u64;
/// Mask which validators participated in multi sign.
#[deprecated(since = "4.0.0", note = "Type has no connection with the SDK")]
pub type ValidatorMask = Vec<bool>;
/// StorageUsage is used to count the amount of storage used by a contract.
pub type StorageUsage = u64;
/// StorageUsageChange is used to count the storage usage within a single contract call.
#[deprecated(since = "4.0.0", note = "Type has no connection with the SDK")]
pub type StorageUsageChange = i64;
/// Nonce for transactions.
#[deprecated(since = "4.0.0", note = "Type has no connection with the SDK")]
pub type Nonce = u64;
/// Height of the block.
pub type BlockHeight = u64;
/// Height of the epoch.
pub type EpochHeight = u64;
/// Shard index, from 0 to NUM_SHARDS - 1.
#[deprecated(since = "4.0.0", note = "Type has no connection with the SDK")]
pub type ShardId = u64;
/// Number of blocks in current group.
#[deprecated(since = "4.0.0", note = "Type has no connection with the SDK")]
pub type NumBlocks = u64;
/// Number of shards in current group.
#[deprecated(since = "4.0.0", note = "Type has no connection with the SDK")]
pub type NumShards = u64;
/// Number of seats of validators (block producer or hidden ones) in current group (settlement).
#[deprecated(since = "4.0.0", note = "Type has no connection with the SDK")]
pub type NumSeats = u64;
/// Block height delta that measures the difference between `BlockHeight`s.
#[deprecated(since = "4.0.0", note = "Type has no connection with the SDK")]
pub type BlockHeightDelta = u64;

#[deprecated(since = "4.0.0", note = "Type has no connection with the SDK")]
pub type GCCount = u64;

#[deprecated(since = "4.0.0", note = "Type has no connection with the SDK")]
pub type PromiseId = Vec<usize>;

#[deprecated(since = "4.0.0", note = "Type has no connection with the SDK")]
pub type ProtocolVersion = u32;
