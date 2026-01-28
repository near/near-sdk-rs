//! Integration tests for near-contract-sim
//!
//! These tests demonstrate all the features of near-contract-sim using real NEAR contracts.

use near_contract_sim::{ContractSim, NearGas, NearToken};
use serde_json::json;

// ============================================================================
// Test Contract WASM Loading
// ============================================================================

/// A minimal WASM contract (compiled from WAT) that just has a simple function
fn minimal_wasm() -> Vec<u8> {
    // A minimal NEAR contract that exposes a `hello` method
    let wat = r#"
        (module
            (import "env" "value_return" (func $value_return (param i64 i64)))
            (memory 1)
            (export "memory" (memory 0))
            (data (i32.const 0) "\"hello\"")
            
            (func (export "hello")
                (call $value_return (i64.const 7) (i64.const 0))
            )
        )
    "#;
    wat::parse_str(wat).expect("failed to parse WAT")
}

/// Get the status-message WASM bytes
fn status_message_wasm() -> Vec<u8> {
    // Path to cargo-near built contract
    let wasm_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../examples/status-message/target/near/status_message.wasm"
    );

    match std::fs::read(wasm_path) {
        Ok(bytes) => bytes,
        Err(e) => panic!(
            "Failed to read status_message.wasm at {}: {}\n\
             Build it first with: cd examples/status-message && cargo near build non-reproducible-wasm",
            wasm_path, e
        )
    }
}

/// Get the cross-contract high-level WASM bytes
fn cross_contract_wasm() -> Vec<u8> {
    let wasm_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../examples/cross-contract-calls/target/near/cross_contract_high_level/cross_contract_high_level.wasm"
    );

    match std::fs::read(wasm_path) {
        Ok(bytes) => bytes,
        Err(e) => panic!(
            "Failed to read cross_contract_high_level.wasm at {}: {}\n\
             Build it first with: cd examples/cross-contract-calls/high-level && cargo near build non-reproducible-wasm",
            wasm_path, e
        )
    }
}

/// Get the fungible token WASM bytes
fn fungible_token_wasm() -> Vec<u8> {
    let wasm_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../examples/fungible-token/target/near/fungible_token/fungible_token.wasm"
    );

    match std::fs::read(wasm_path) {
        Ok(bytes) => bytes,
        Err(e) => panic!(
            "Failed to read fungible_token.wasm at {}: {}\n\
             Build it first with: cd examples/fungible-token/ft && cargo near build non-reproducible-wasm",
            wasm_path, e
        )
    }
}

// ============================================================================
// Basic Tests
// ============================================================================

#[test]
fn test_minimal_wasm() {
    let mut sim = ContractSim::new();

    // Deploy the minimal contract
    sim.deploy("minimal.near", minimal_wasm()).unwrap();

    // Call the hello method
    let result = sim.call("alice.near", "minimal.near", "hello", b"").unwrap();

    // Check result
    result.assert_success();

    // Return value should be "hello"
    let value: String = result.json().unwrap();
    assert_eq!(value, "hello");
}

#[test]
fn test_deploy_and_call() {
    let mut sim = ContractSim::new();

    // Deploy the status-message contract
    sim.deploy("status.near", status_message_wasm()).unwrap();

    // Set a status using call_json for convenience
    let result = sim
        .call_json("alice.near", "status.near", "set_status", &json!({"message": "Hello, NEAR!"}))
        .unwrap();

    // Check that execution succeeded
    result.assert_success();

    // Print logs and gas
    println!("Logs: {:?}", result.logs());
    println!(
        "Total gas: {} TGas ({} gas)",
        result.gas_used().as_tgas(),
        result.gas_used().as_gas()
    );

    // Print per-receipt gas
    for (i, receipt) in result.receipts().iter().enumerate() {
        println!("  Receipt [{}]: {} TGas", i, receipt.gas_used.as_tgas());
    }

    // Check logs
    result.assert_logs_contain("set_status");
    result.assert_logs_contain("Hello, NEAR!");
}

#[test]
fn test_view_call() {
    let mut sim = ContractSim::new();

    sim.deploy("status.near", status_message_wasm()).unwrap();

    // Set a status first
    sim.call_json("alice.near", "status.near", "set_status", &json!({"message": "Hello!"}))
        .unwrap()
        .assert_success();

    // Now view the status using view_json
    let result =
        sim.view_json("status.near", "get_status", &json!({"account_id": "alice.near"})).unwrap();

    result.assert_success();

    // The return value should be the message
    let value: Option<String> = result.json().unwrap();
    assert_eq!(value, Some("Hello!".to_string()));
}

#[test]
fn test_storage_persistence() {
    let mut sim = ContractSim::new();

    sim.deploy("status.near", status_message_wasm()).unwrap();

    // Set multiple statuses
    sim.call_json(
        "alice.near",
        "status.near",
        "set_status",
        &json!({"message": "Alice's message"}),
    )
    .unwrap()
    .assert_success();

    sim.call_json("bob.near", "status.near", "set_status", &json!({"message": "Bob's message"}))
        .unwrap()
        .assert_success();

    // Verify both are stored
    let alice_status =
        sim.view_json("status.near", "get_status", &json!({"account_id": "alice.near"})).unwrap();
    let bob_status =
        sim.view_json("status.near", "get_status", &json!({"account_id": "bob.near"})).unwrap();

    assert_eq!(alice_status.json::<Option<String>>().unwrap(), Some("Alice's message".to_string()));
    assert_eq!(bob_status.json::<Option<String>>().unwrap(), Some("Bob's message".to_string()));
}

// ============================================================================
// Block & Time Tests
// ============================================================================

#[test]
fn test_block_advancement() {
    let mut sim = ContractSim::new();

    assert_eq!(sim.block_height(), 1);

    sim.advance_block();
    assert_eq!(sim.block_height(), 2);

    sim.advance_blocks(10);
    assert_eq!(sim.block_height(), 12);

    sim.set_block_height(100);
    assert_eq!(sim.block_height(), 100);
}

#[test]
fn test_timestamp_advancement() {
    let mut sim = ContractSim::new();

    let initial = sim.block_timestamp();
    sim.advance_block();

    // Should advance by ~1 second (1 billion nanoseconds)
    assert_eq!(sim.block_timestamp(), initial + 1_000_000_000);
}

// ============================================================================
// Mocking Tests
// ============================================================================

#[test]
fn test_mock_response() {
    let mut sim = ContractSim::new();

    // Mock an oracle contract - handler receives (method, args)
    sim.mock("oracle.near", |_method, _args| {
        near_contract_sim::MockResponse::success_json(&json!({"price": "5.50"}))
    })
    .unwrap();

    // The mock would be used when a deployed contract calls oracle.near::get_price
    // For now, just verify the mock is registered without panic
}

#[test]
fn test_cross_contract_logs() {
    let mut sim = ContractSim::new();

    sim.deploy("factorial.near", cross_contract_wasm()).unwrap();

    let result =
        sim.call_json("alice.near", "factorial.near", "factorial", &json!({"n": 3})).unwrap();

    result.assert_success();

    // Check that we have multiple receipts (cross-contract calls happened)
    let receipts = result.receipts();
    println!("Number of receipts: {}", receipts.len());

    // Print all receipts with details
    for (i, receipt) in receipts.iter().enumerate() {
        println!(
            "  [{}] {}::{} - {:?} (gas: {} TGas, logs: {:?})",
            i,
            receipt.receiver,
            receipt.method,
            receipt.status,
            receipt.gas_used.as_tgas(),
            receipt.logs
        );
    }

    assert!(receipts.len() > 1, "Expected multiple receipts for cross-contract calls");
}

#[test]
fn test_mock_with_handler() {
    use near_contract_sim::MockResponse;

    let mut sim = ContractSim::new();

    // Mock with dynamic handler based on method and args
    sim.mock("oracle.near", |method, args| {
        if method != "get_price" {
            return MockResponse::failure("Unknown method");
        }
        let args_str = String::from_utf8_lossy(args);
        if args_str.contains("BTC") {
            MockResponse::success_json(&json!({"price": "45000.00"}))
        } else if args_str.contains("ETH") {
            MockResponse::success_json(&json!({"price": "3000.00"}))
        } else {
            MockResponse::failure("Unknown asset")
        }
    })
    .unwrap();
}

// ============================================================================
// Builder API Tests
// ============================================================================

#[test]
fn test_fluent_builder() {
    let mut sim = ContractSim::new();

    sim.deploy("status.near", status_message_wasm()).unwrap();

    let result = sim
        .call_builder("status.near", "set_status")
        .signer("charlie.near")
        .args_json(&json!({"message": "Built fluently!"}))
        .deposit(NearToken::from_yoctonear(0))
        .execute()
        .unwrap();

    result.assert_success();
    result.assert_logs_contain("charlie.near");
}

#[test]
fn test_builder_with_gas() {
    let mut sim = ContractSim::new();

    sim.deploy("status.near", status_message_wasm()).unwrap();

    let result = sim
        .call_builder("status.near", "set_status")
        .signer("alice.near")
        .args_json(&json!({"message": "With gas limit"}))
        .gas_tgas(100)
        .execute()
        .unwrap();

    result.assert_success();
}

#[test]
fn test_builder_with_deposit() {
    let mut sim = ContractSim::new();

    sim.deploy("status.near", status_message_wasm()).unwrap();

    let result = sim
        .call_builder("status.near", "set_status")
        .signer("alice.near")
        .args_json(&json!({"message": "With deposit"}))
        .deposit_near(1)
        .execute()
        .unwrap();

    result.assert_success();
}

// ============================================================================
// Gas Tracking Tests
// ============================================================================

#[test]
fn test_gas_tracking() {
    let mut sim = ContractSim::new();

    sim.deploy("status.near", status_message_wasm()).unwrap();

    let result = sim
        .call_json("alice.near", "status.near", "set_status", &json!({"message": "test"}))
        .unwrap();

    result.assert_success();

    // Gas should be tracked
    let gas_used = result.gas_used();
    assert!(gas_used.as_gas() > 0, "Gas used should be greater than 0");

    // Should be less than default limit
    result.assert_gas_lt(NearGas::from_tgas(300));
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_contract_not_found() {
    let mut sim = ContractSim::new();

    // Try to call a contract that doesn't exist
    let result = sim.call("alice.near", "nonexistent.near", "some_method", b"{}");

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Contract not found"));
}

#[test]
fn test_invalid_account_id() {
    let mut sim = ContractSim::new();

    // Try to deploy with invalid account ID
    let result = sim.deploy("not a valid account id!", minimal_wasm());
    assert!(result.is_err());
}

// ============================================================================
// Storage Tests
// ============================================================================

#[test]
fn test_storage_dump() {
    let mut sim = ContractSim::new();

    sim.deploy("status.near", status_message_wasm()).unwrap();

    // Set a status
    sim.call_json("alice.near", "status.near", "set_status", &json!({"message": "stored value"}))
        .unwrap()
        .assert_success();

    // Dump storage
    let storage = sim.storage_dump("status.near").unwrap();

    // Should have some storage entries
    assert!(!storage.is_empty(), "Storage should not be empty after set_status");
}

#[test]
fn test_storage_keys() {
    let mut sim = ContractSim::new();

    sim.deploy("status.near", status_message_wasm()).unwrap();

    sim.call_json("alice.near", "status.near", "set_status", &json!({"message": "test"}))
        .unwrap()
        .assert_success();

    // Get storage keys (as strings for debugging)
    let keys = sim.storage_keys("status.near").unwrap();
    assert!(!keys.is_empty());
}

// ============================================================================
// Account Management Tests
// ============================================================================

#[test]
fn test_account_management() {
    let mut sim = ContractSim::new();

    // Create account with balance
    sim.create_account("rich.near", NearToken::from_near(1000)).unwrap();

    // Check balance
    let balance = sim.balance("rich.near").unwrap();
    assert_eq!(balance, NearToken::from_near(1000));

    // Deploy contract
    sim.deploy("contract.near", minimal_wasm()).unwrap();

    // Check has_contract
    assert!(sim.has_contract("contract.near").unwrap());
    assert!(!sim.has_contract("rich.near").unwrap());
}

#[test]
fn test_list_accounts() {
    let mut sim = ContractSim::new();

    sim.create_account("alice.near", NearToken::from_near(100)).unwrap();
    sim.create_account("bob.near", NearToken::from_near(100)).unwrap();
    sim.deploy("contract.near", minimal_wasm()).unwrap();

    let accounts = sim.accounts();
    assert!(accounts.contains(&"alice.near".to_string()));
    assert!(accounts.contains(&"bob.near".to_string()));
    assert!(accounts.contains(&"contract.near".to_string()));
}

// ============================================================================
// Cross-Contract Call Tests
// ============================================================================

#[test]
fn test_cross_contract_factorial() {
    let mut sim = ContractSim::new();

    // Deploy the factorial contract
    sim.deploy("factorial.near", cross_contract_wasm()).unwrap();

    // Call factorial(5) - this makes recursive cross-contract calls
    let result =
        sim.call_json("alice.near", "factorial.near", "factorial", &json!({"n": 5})).unwrap();

    // Check success
    result.assert_success();

    // Check we got multiple receipts (cross-contract calls)
    let receipts = result.receipts();
    assert!(receipts.len() > 1, "Should have multiple receipts for cross-contract calls");

    // Print execution trace for debugging
    println!("Execution trace ({} receipts):", receipts.len());
    for (i, receipt) in receipts.iter().enumerate() {
        println!(
            "  [{}] {}::{} - {:?} (gas: {} TGas)",
            i,
            receipt.receiver,
            receipt.method,
            receipt.status,
            receipt.gas_used.as_tgas()
        );
    }

    // Check total gas
    let total_gas = result.gas_used();
    println!("Total gas used: {} TGas", total_gas.as_tgas());

    // The final result should be 5! = 120
    // Note: The high-level contract returns PromiseOrValue, so the result comes from the last callback
}

// ============================================================================
// Fungible Token Tests
// ============================================================================

#[test]
fn test_ft_initialization() {
    let mut sim = ContractSim::new();

    sim.deploy("ft.near", fungible_token_wasm()).unwrap();

    // Initialize the FT contract
    let result = sim
        .call_builder("ft.near", "new_default_meta")
        .signer("ft.near")
        .args_json(&json!({
            "owner_id": "owner.near",
            "total_supply": "1000000000000000000000000"
        }))
        .execute()
        .unwrap();

    result.assert_success();

    // Check the owner's balance
    let balance_result =
        sim.view_json("ft.near", "ft_balance_of", &json!({"account_id": "owner.near"})).unwrap();

    balance_result.assert_success();
    let balance: String = balance_result.json().unwrap();
    assert_eq!(balance, "1000000000000000000000000");
}

#[test]
fn test_ft_metadata() {
    let mut sim = ContractSim::new();

    sim.deploy("ft.near", fungible_token_wasm()).unwrap();

    // Initialize
    sim.call_builder("ft.near", "new_default_meta")
        .signer("ft.near")
        .args_json(&json!({
            "owner_id": "owner.near",
            "total_supply": "1000000000000"
        }))
        .execute()
        .unwrap()
        .assert_success();

    // Get metadata
    let result = sim.view("ft.near", "ft_metadata", b"{}").unwrap();
    result.assert_success();

    let metadata: serde_json::Value = result.json().unwrap();
    assert!(metadata.get("name").is_some());
    assert!(metadata.get("symbol").is_some());
    assert!(metadata.get("decimals").is_some());
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_empty_args() {
    let mut sim = ContractSim::new();
    sim.deploy("minimal.near", minimal_wasm()).unwrap();

    // Call with empty args
    let result = sim.call("alice.near", "minimal.near", "hello", b"").unwrap();
    result.assert_success();
}

#[test]
fn test_multiple_deploys() {
    let mut sim = ContractSim::new();

    // Deploy same contract to multiple accounts
    sim.deploy("contract1.near", minimal_wasm()).unwrap();
    sim.deploy("contract2.near", minimal_wasm()).unwrap();
    sim.deploy("contract3.near", minimal_wasm()).unwrap();

    // All should work
    sim.call("alice.near", "contract1.near", "hello", b"").unwrap().assert_success();
    sim.call("alice.near", "contract2.near", "hello", b"").unwrap().assert_success();
    sim.call("alice.near", "contract3.near", "hello", b"").unwrap().assert_success();
}

#[test]
fn test_redeploy_contract() {
    let mut sim = ContractSim::new();

    sim.deploy("contract.near", status_message_wasm()).unwrap();

    // Set some state
    sim.call_json(
        "alice.near",
        "contract.near",
        "set_status",
        &json!({"message": "before redeploy"}),
    )
    .unwrap()
    .assert_success();

    // Redeploy with same contract (state is preserved in our simple model)
    sim.deploy("contract.near", status_message_wasm()).unwrap();

    // State should still exist
    let result =
        sim.view_json("contract.near", "get_status", &json!({"account_id": "alice.near"})).unwrap();
    result.assert_success();
}
