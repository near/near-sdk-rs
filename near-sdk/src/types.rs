pub type AccountId = String;
pub type PublicKey = Vec<u8>;
pub type BlockHeight = u64;
pub type EpochHeight = u64;
pub type Balance = u128;
pub type Gas = u64;
pub type PromiseIndex = u64;
pub type ReceiptIndex = u64;
pub type IteratorIndex = u64;
pub type StorageUsage = u64;
pub type ProtocolVersion = u32;

/// When there is a callback attached to one or more contract calls the execution results of these
/// calls are available to the contract invoked through the callback.
#[derive(Debug, PartialEq)]
pub enum PromiseResult {
    /// Current version of the protocol never returns `PromiseResult::NotReady`.
    NotReady,
    Successful(Vec<u8>),
    Failed,
}
