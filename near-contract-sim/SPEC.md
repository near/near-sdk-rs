# near-contract-sim Specification

## Overview

`near-contract-sim` is a lightweight, in-process multi-contract testing runtime for NEAR smart contracts. It fills the gap between fast-but-limited unit tests and slow-but-complete integration tests (near-workspaces).

## Problem Statement

Currently, NEAR developers have two testing options:

1. **Unit tests (`testing_env!`)**: Fast, but cannot execute cross-contract calls. Only records that receipts were created.

2. **Integration tests (`near-workspaces`)**: Full cross-contract support, but slow (compiles to WASM, spawns sandbox node, IPC overhead).

There's no middle ground for developers who want to:
- Test cross-contract call flows quickly
- Iterate rapidly during development
- Test callback logic without full sandbox overhead
- Mock external contracts while testing their own

## Goals

1. **Fast**: Run entirely in-process, no external binaries or network calls
2. **Simple**: Minimal API surface, easy to understand and use
3. **Accurate**: Use real `near-vm-runner` for execution, matching production behavior
4. **Flexible**: Support both real contract execution and mocked responses
5. **Composable**: Work alongside existing `testing_env!` and `near-workspaces`

## Non-Goals

1. **100% protocol fidelity**: We don't aim to replicate every edge case of the full runtime
2. **Consensus/networking**: Single-threaded, deterministic execution only
3. **Account creation/deletion**: Focus on contract execution, not account management
4. **Access key management**: Not needed for most contract testing scenarios
5. **Replacing near-workspaces**: This is for fast iteration; use workspaces for final integration tests

---

## End-User Interface

### Basic Setup

```rust
use near_contract_sim::{ContractSim, SimAccount};
use near_sdk::NearToken;
use near_gas::NearGas;

#[test]
fn test_my_contract() {
    // Create a new simulation environment
    let mut sim = ContractSim::new();
    
    // Deploy contracts
    sim.deploy(
        "my-contract.near",
        include_bytes!("../../target/wasm32-unknown-unknown/release/my_contract.wasm"),
    );
    
    // Execute a call
    let result = sim.call(
        "alice.near",           // signer/predecessor
        "my-contract.near",     // receiver
        "my_method",            // method name
        br#"{"arg": "value"}"#, // JSON args
    );
    
    assert!(result.is_ok());
    assert_eq!(result.unwrap_json::<String>(), "expected_result");
}
```

### Fluent Builder API

```rust
let result = sim
    .call("my-contract.near", "transfer")
    .signer("alice.near")
    .args_json(&json!({"to": "bob.near", "amount": "100"}))
    .deposit(NearToken::from_near(1))
    .gas(NearGas::from_tgas(50))
    .execute();
```

### Cross-Contract Calls

When a contract creates receipts (cross-contract calls), they are automatically executed:

```rust
// Deploy both contracts
sim.deploy("token.near", token_wasm);
sim.deploy("dex.near", dex_wasm);

// This call might trigger: dex -> token -> dex (callback)
let result = sim.call("alice.near", "dex.near", "swap", args);

// All cross-contract calls are executed, result contains final outcome
assert!(result.is_ok());

// Inspect the full execution trace
for (i, receipt) in result.receipts().iter().enumerate() {
    println!("Receipt {}: {} -> {}::{}", 
        i, receipt.predecessor, receipt.receiver, receipt.method);
}
```

### Mocking External Contracts

Mock contracts you don't control or don't want to execute:

```rust
// Deploy your contract
sim.deploy("my-dex.near", dex_wasm);

// Mock external oracle
sim.mock("oracle.near", "get_price", |args| {
    MockResponse::Success(br#"{"price": "5.50"}"#.to_vec())
});

// Or with a static response
sim.mock_response("oracle.near", "get_price", br#"{"price": "5.50"}"#);

// Or simulate failure
sim.mock_failure("oracle.near", "get_price", "Service unavailable");

// Or simulate a panic
sim.mock_panic("oracle.near", "get_price", "Unexpected error!");

// Now test your contract - oracle calls return mocked values
let result = sim.call("alice.near", "my-dex.near", "swap", args);
```

### Conditional Mocking

```rust
// Mock based on input arguments
sim.mock("token.near", "ft_balance_of", |args: Value| {
    let account_id = args["account_id"].as_str().unwrap();
    match account_id {
        "alice.near" => MockResponse::Success(br#""1000""#.to_vec()),
        "bob.near" => MockResponse::Success(br#""500""#.to_vec()),
        _ => MockResponse::Success(br#""0""#.to_vec()),
    }
});
```

### View Calls

```rust
// View calls are read-only and don't modify state
let balance = sim.view("token.near", "ft_balance_of", 
    json!({"account_id": "alice.near"}));

assert_eq!(balance.unwrap_json::<String>(), "1000");
```

### State Inspection

```rust
// Check account balance
let balance = sim.account_balance("alice.near");

// Read raw storage
let value = sim.storage_read("my-contract.near", b"STATE");

// Dump all storage for debugging
let storage = sim.storage_dump("my-contract.near");
for (key, value) in storage {
    println!("{:?} = {:?}", key, value);
}
```

### Time Manipulation

```rust
// Advance block height
sim.advance_block();
sim.advance_blocks(100);

// Set specific block height
sim.set_block_height(12345);

// Set block timestamp (nanoseconds)
sim.set_block_timestamp(1_700_000_000_000_000_000);

// Advance time by duration
sim.advance_time(std::time::Duration::from_secs(3600)); // 1 hour
```

### Account Setup

```rust
// Create accounts with initial balance
sim.create_account("alice.near", NearToken::from_near(100));
sim.create_account("bob.near", NearToken::from_near(50));

// Or use the builder
sim.account("alice.near")
    .balance(NearToken::from_near(100))
    .create();
```

### Gas Profiling

```rust
let result = sim.call("alice.near", "contract.near", "expensive_method", args);

// Get gas usage
println!("Gas used: {}", result.gas_used());

// Detailed breakdown
let profile = result.gas_profile();
println!("Compute: {}", profile.compute);
println!("Storage reads: {}", profile.storage_read);
println!("Storage writes: {}", profile.storage_write);
```

### Execution Traces

```rust
let result = sim.call("alice.near", "dex.near", "swap", args);

// Get all logs across all receipts
for log in result.all_logs() {
    println!("LOG: {}", log);
}

// Get execution trace
for receipt in result.execution_trace() {
    println!("{} -> {}::{}", 
        receipt.predecessor, 
        receipt.receiver, 
        receipt.method);
    println!("  Status: {:?}", receipt.status);
    println!("  Gas: {}", receipt.gas_used);
    for log in &receipt.logs {
        println!("  LOG: {}", log);
    }
}
```

### Error Handling

```rust
let result = sim.call("alice.near", "contract.near", "will_fail", args);

match result {
    Ok(outcome) => {
        println!("Success: {:?}", outcome.return_value());
    }
    Err(SimError::ExecutionError { message, .. }) => {
        println!("Contract panicked: {}", message);
    }
    Err(SimError::ContractNotFound { account_id }) => {
        println!("No contract at {}", account_id);
    }
    Err(SimError::MethodNotFound { method, .. }) => {
        println!("Method {} not found", method);
    }
    Err(SimError::OutOfGas { used, limit }) => {
        println!("Ran out of gas: {} / {}", used, limit);
    }
    Err(e) => {
        println!("Other error: {:?}", e);
    }
}
```

### Assertions

```rust
// Fluent assertion helpers
result.assert_success();
result.assert_failure();
result.assert_failure_contains("insufficient balance");
result.assert_logs_contain("Transfer successful");
result.assert_gas_used_lt(NearGas::from_tgas(10));

// JSON result assertions
result.assert_json_eq(&json!({"status": "ok"}));
```

---

## Type Definitions

### Core Types

```rust
/// The main simulation runtime
pub struct ContractSim { ... }

/// Result of a contract execution (may include multiple receipts)
pub struct SimResult { ... }

/// A single receipt execution outcome
pub struct ReceiptOutcome {
    pub predecessor: AccountId,
    pub receiver: AccountId,
    pub method: String,
    pub args: Vec<u8>,
    pub deposit: NearToken,
    pub gas_used: NearGas,
    pub status: ExecutionStatus,
    pub logs: Vec<String>,
    pub return_value: Option<Vec<u8>>,
}

/// Execution status
pub enum ExecutionStatus {
    Success,
    Failure(String),
    Panic(String),
}

/// Mock response types
pub enum MockResponse {
    Success(Vec<u8>),
    Failure(String),
    Panic(String),
}

/// Simulation errors
pub enum SimError {
    ContractNotFound { account_id: AccountId },
    MethodNotFound { account_id: AccountId, method: String },
    ExecutionError { account_id: AccountId, method: String, message: String },
    OutOfGas { used: NearGas, limit: NearGas },
    DeserializationError { message: String },
    InvalidAccountId { id: String },
}
```

### Builder Types

```rust
/// Builder for call execution
pub struct CallBuilder<'a> { ... }

impl<'a> CallBuilder<'a> {
    pub fn signer(self, account_id: &str) -> Self;
    pub fn args(self, args: &[u8]) -> Self;
    pub fn args_json<T: Serialize>(self, args: &T) -> Self;
    pub fn deposit(self, amount: NearToken) -> Self;
    pub fn gas(self, gas: NearGas) -> Self;
    pub fn execute(self) -> Result<SimResult, SimError>;
}
```

---

## Configuration

### Default Configuration

```rust
// Uses sensible defaults
let sim = ContractSim::new();
```

### Custom Configuration

```rust
let sim = ContractSim::builder()
    .default_gas(NearGas::from_tgas(300))
    .default_deposit(NearToken::from_yoctonear(0))
    .initial_balance(NearToken::from_near(1000))
    .block_height(1)
    .block_timestamp(0)
    .build();
```

---

## Comparison with Existing Tools

| Feature | `testing_env!` | `near-contract-sim` | `near-workspaces` |
|---------|---------------|---------------------|-------------------|
| Speed | ⚡ Very fast | ⚡ Fast | 🐢 Slow |
| Cross-contract calls | ❌ Records only | ✅ Executes | ✅ Executes |
| Mocking | ❌ No | ✅ Yes | ❌ No |
| Real WASM execution | ❌ Native | ✅ WASM | ✅ WASM |
| State persistence | Per-test | Per-sim | Per-sandbox |
| Setup complexity | Low | Low | Medium |
| Protocol fidelity | Low | Medium | High |

---

## Limitations

1. **WASM only**: Contracts must be compiled to WASM (no native execution)
2. **No parallel execution**: Receipts are processed sequentially
3. **Simplified gas model**: Uses `near-vm-runner` gas, but no burnt gas distribution
4. **No protocol upgrades**: Single protocol version
5. **No validators**: `env::validator_*` functions return empty/default values
6. **Simplified storage**: In-memory HashMap, no trie structure

---

## Future Considerations

1. **Snapshot/restore**: Save and restore simulation state
2. **Fork from testnet**: Initialize state from real network
3. **Native execution**: Run contracts without WASM compilation for speed
4. **Parallel receipts**: Execute independent receipts in parallel
5. **Integration with testing_env!**: Use sim as backend for existing macro

---

## Success Criteria

1. **10x faster** than near-workspaces for typical test suites
2. **Zero external dependencies** (no sandbox binary, no network)
3. **< 1000 lines of code** for core implementation
4. **100% compatible** with existing contract WASM files
5. **Clear documentation** with examples for common use cases
