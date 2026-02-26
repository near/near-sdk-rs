//! # Sharded Fungible Token contracts
//!
//! The design is highly inspired by [Jetton](https://docs.ton.org/v3/gu!delines/dapps/asset-processing/jettons#jetton-architecture)
//! standard except for following differences:
//! * Unlike TVM, Near doesn't support [message bouncing](https://docs.ton.org/v3/documentation/smart-contracts/transaction-fees/forward-fees#message-bouncing),
//!   so instead we can schedule callbacks, which gives more control over
//!   handling of failed cross-contract calls.
//! * TVM doesn't differentiate between gas and attached deposit, while
//!   in Near they are not coupled, which removes some complexities.
//!
//! ## Events
//!
//! Similar to Jetton standard, there is no logging of such events as
//! `sft_transfer`, `sft_mint` or `sft_burn` as it simply wouldn't bring any
//! value for indexers. Even if we do emit these events, indexers are still
//! forced to track `sft_transfer` function calls to not-yet-existing
//! wallet-contracts, which will emit these events.
//! However, to properly track these cross-contract calls they would need
//! parse function names (i.e. `sft_transfer()`, `sft_receive()`, `sft_burn()`
//! and `sft_resolve()`) and their args, while this information combined with
//! receipt status already contains all necessary info for indexing.

pub mod minter;
pub mod receiver;
pub mod wallet;
