//! # near-contract-sim
//!
//! Lightweight multi-contract testing runtime for NEAR smart contracts.
//!
//! Fills the gap between fast-but-limited unit tests (`testing_env!`) and
//! slow-but-complete integration tests (`near-workspaces`).
//!
//! ## Features
//!
//! - **Fast**: Runs entirely in-process, no external sandbox binary
//! - **Cross-contract calls**: Actually executes calls between contracts
//! - **Mocking**: Mock external contracts for faster/simpler tests
//! - **Gas tracking**: Real gas metering via `near-vm-runner`
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use near_contract_sim::ContractSim;
//! use serde_json::json;
//!
//! #[test]
//! fn test_my_contract() {
//!     let mut sim = ContractSim::new();
//!     
//!     // Deploy (use: cargo near build non-reproducible-wasm)
//!     sim.deploy("contract.near", include_bytes!("path/to/contract.wasm")).unwrap();
//!     
//!     // Call
//!     let result = sim.call_json("alice.near", "contract.near", "method", &json!({"arg": 1})).unwrap();
//!     result.assert_success();
//!     
//!     // Parse result
//!     let value: u32 = result.json().unwrap();
//! }
//! ```
//!
//! ## Mocking
//!
//! ```rust,ignore
//! use near_contract_sim::{ContractSim, MockResponse};
//!
//! sim.mock("oracle.near", |method, args| {
//!     match method {
//!         "get_price" => MockResponse::success_json(&json!({"usd": "5.50"})),
//!         _ => MockResponse::panic("Unknown method"),
//!     }
//! }).unwrap();
//! ```

mod error;
mod executor;
mod outcome;
mod sim;
mod state;

pub use error::{SimError, SimResult};
pub use executor::{Receipt, ReceiptId};
pub use outcome::{CallOutcome, ExecutionStatus, GasProfile, MockResponse, ReceiptOutcome};
pub use sim::{CallBuilder, ContractSim};

// Re-export common types
pub use near_account_id::AccountId;
pub use near_gas::NearGas;
pub use near_token::NearToken;
