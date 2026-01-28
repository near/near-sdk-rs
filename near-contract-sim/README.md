# near-contract-sim

**Lightweight multi-contract testing runtime for NEAR smart contracts.**

This crate fills the gap between:
- **Unit tests (`testing_env!`)**: Fast but can't execute real cross-contract calls
- **Integration tests (`near-workspaces`)**: Full cross-contract support but slow (spawns sandbox node)

`near-contract-sim` gives you the best of both worlds: **fast in-process testing with actual WASM execution and cross-contract calls**.

## Features

- ⚡ **Fast**: Runs entirely in-process, no external sandbox binary needed
- 🔗 **Cross-contract calls**: Actually executes calls between multiple contracts
- 🎭 **Mocking**: Mock external contracts for faster/simpler tests
- ⛽ **Gas tracking**: Real gas metering via `near-vm-runner`
- 🧪 **Simple API**: Clean, fluent interface for writing tests

## Installation

Add to your `Cargo.toml`:

```toml
[dev-dependencies]
near-contract-sim = "0.1"
```

## Quick Start

```rust
use near_contract_sim::ContractSim;

#[test]
fn test_my_contract() {
    let mut sim = ContractSim::new();
    
    // Deploy a contract
    sim.deploy("my-contract.near", include_bytes!("../target/near/my_contract.wasm")).unwrap();
    
    // Call a method
    let result = sim.call(
        "alice.near",           // signer
        "my-contract.near",     // receiver
        "my_method",            // method
        br#"{"arg": "value"}"#, // JSON args
    ).unwrap();
    
    // Assert success
    result.assert_success();
    
    // Check logs
    result.assert_logs_contain("expected log message");
    
    // Parse return value
    let value: String = result.json().unwrap();
}
```

## Building Contracts

Contracts should be built with `cargo near` for proper WASM optimization:

```bash
cd my-contract
cargo near build non-reproducible-wasm
```

This produces optimized WASM at `target/near/my_contract.wasm`.

## API Overview

### Deploy & Call

```rust
// Deploy from bytes
sim.deploy("contract.near", wasm_bytes)?;

// Deploy from file
sim.deploy_file("contract.near", "path/to/contract.wasm")?;

// Simple call
let result = sim.call("signer.near", "contract.near", "method", b"{}")?;

// Call with JSON args (automatically serialized)
let result = sim.call_json("signer.near", "contract.near", "method", &json!({"key": "value"}))?;

// View call (read-only)
let result = sim.view("contract.near", "get_value", b"{}")?;
let value: u64 = result.json()?;
```

### Fluent Builder API

For more control, use the builder pattern:

```rust
let result = sim
    .call_builder("contract.near", "transfer")
    .signer("alice.near")
    .args_json(&json!({"to": "bob.near", "amount": "100"}))
    .deposit_near(1)           // Attach 1 NEAR
    .gas_tgas(100)             // Use 100 TGas
    .execute()?;

result.assert_success();
```

### Mocking External Contracts

Don't want to deploy a real oracle? Mock it!

```rust
// Static mock response
sim.mock_json("oracle.near", "get_price", &json!({"price": "5.50"}))?;

// Dynamic mock with handler
sim.mock("oracle.near", "get_price", |args| {
    let request: PriceRequest = serde_json::from_slice(args).unwrap();
    if request.asset == "BTC" {
        MockResponse::success_json(&json!({"price": "45000.00"}))
    } else {
        MockResponse::failure("Unknown asset")
    }
})?;

// Mock failure
sim.mock_failure("oracle.near", "get_price", "Service unavailable")?;
```

### Time Control

```rust
// Advance blocks
sim.advance_block();
sim.advance_blocks(10);

// Set specific block
sim.set_block_height(1000);
sim.set_block_timestamp(1_000_000_000); // nanoseconds
```

### Inspecting Results

```rust
let result = sim.call(...)?;

// Check status
assert!(result.is_success());
result.assert_success();
result.assert_failure();
result.assert_failure_contains("expected error");

// Get return value
let bytes: Option<&[u8]> = result.return_value();
let value: MyType = result.json()?;

// Get logs
let logs: Vec<&str> = result.logs();
result.assert_logs_contain("transferred");

// Gas usage
let gas_used: NearGas = result.gas_used();
result.assert_gas_lt(NearGas::from_tgas(50));

// Execution trace (for cross-contract calls)
for receipt in result.receipts() {
    println!("{}::{} - {:?}", receipt.receiver, receipt.method, receipt.status);
}
```

### Storage Inspection

```rust
// Dump all storage
let storage = sim.storage_dump("contract.near")?;

// Read specific key
let value = sim.storage_read("contract.near", b"STATE")?;

// List keys (for debugging)
let keys = sim.storage_keys("contract.near")?;
```

## Cross-Contract Calls

Cross-contract calls work automatically! When a contract makes a promise to call another contract, `near-contract-sim` executes it:

```rust
// Deploy both contracts
sim.deploy("factorial.near", factorial_wasm)?;

// Call factorial(5) - this recursively calls itself
let result = sim.call_json("user.near", "factorial.near", "factorial", &json!({"n": 5}))?;

result.assert_success();

// See the execution trace
println!("Receipts: {}", result.receipts().len());
for receipt in result.receipts() {
    println!("  {}::{}", receipt.receiver, receipt.method);
}
```

## Full Example: Fungible Token

```rust
use near_contract_sim::ContractSim;
use serde_json::json;

#[test]
fn test_ft_transfer() {
    let mut sim = ContractSim::new();
    
    // Deploy FT contract
    sim.deploy("ft.near", include_bytes!("../target/near/ft.wasm")).unwrap();
    
    // Initialize with owner
    sim.call_builder("ft.near", "new_default_meta")
        .signer("ft.near")
        .args_json(&json!({
            "owner_id": "owner.near",
            "total_supply": "1000000000000000000000000"
        }))
        .execute()
        .unwrap()
        .assert_success();
    
    // Register recipient
    sim.call_builder("ft.near", "storage_deposit")
        .signer("recipient.near")
        .args_json(&json!({"account_id": "recipient.near"}))
        .deposit_near(1)
        .execute()
        .unwrap()
        .assert_success();
    
    // Transfer tokens
    sim.call_builder("ft.near", "ft_transfer")
        .signer("owner.near")
        .args_json(&json!({
            "receiver_id": "recipient.near",
            "amount": "100000000000000000000"
        }))
        .deposit_yocto(1)  // Requires 1 yocto for security
        .execute()
        .unwrap()
        .assert_success();
    
    // Check balance
    let balance = sim
        .view_json("ft.near", "ft_balance_of", &json!({"account_id": "recipient.near"}))
        .unwrap();
    
    let amount: String = balance.json().unwrap();
    assert_eq!(amount, "100000000000000000000");
}
```

## Comparison with Alternatives

| Feature | `testing_env!` | `near-workspaces` | `near-contract-sim` |
|---------|---------------|-------------------|---------------------|
| Speed | ⚡ Very Fast | 🐢 Slow | ⚡ Fast |
| Cross-contract calls | ❌ No | ✅ Yes | ✅ Yes |
| Real WASM execution | ❌ No | ✅ Yes | ✅ Yes |
| Gas metering | ⚠️ Limited | ✅ Yes | ✅ Yes |
| Requires sandbox | ❌ No | ✅ Yes | ❌ No |
| Mocking | Manual | ❌ No | ✅ Built-in |

## When to Use What

- **Unit tests (`testing_env!`)**: Testing single contract logic in isolation
- **`near-contract-sim`**: Testing multi-contract interactions, fast iteration
- **`near-workspaces`**: Final integration tests, testing with real blockchain state

## License

MIT OR Apache-2.0
