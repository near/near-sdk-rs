# What We Built

## Overview

`near-contract-sim` is a lightweight multi-contract testing runtime that fills the gap between:
- **Unit tests** (`testing_env!`) - fast but no cross-contract calls
- **Integration tests** (`near-workspaces`) - full fidelity but slow (spawns sandbox process)

## Architecture

### The Core Insight

When a receipt "forwards" to another (returns a promise), we just rewrite dependencies in pending receipts. No need for complex bidirectional tracking.

```
┌─────────────────────────────────────────────────────────────┐
│  ContractSim                                                 │
│  ├── accounts: HashMap<AccountId, AccountState>             │
│  │   └── AccountState { code, storage }                     │
│  ├── mock_handlers: HashMap<AccountId, MockHandler>         │
│  └── execute_all() - the main loop                          │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  Execution Loop (simplified)                                 │
│                                                              │
│  while let Some(receipt) = pop_ready(&mut pending, &results) │
│      let result = execute(receipt, promise_results);         │
│      pending.extend(result.new_receipts);                    │
│                                                              │
│      match result.completion {                               │
│          Value(data) => results.insert(id, Successful(data)) │
│          Forward(target) => rewrite_deps(id → target)        │
│          Failed => results.insert(id, Failed)                │
│      }                                                       │
│  }                                                           │
└─────────────────────────────────────────────────────────────┘
```

### File Structure

```
near-contract-sim/src/
├── lib.rs        (64 lines)   - Module exports
├── sim.rs        (521 lines)  - Main ContractSim API + execution loop
├── executor.rs   (264 lines)  - WASM execution via near-vm-runner
├── outcome.rs    (229 lines)  - CallOutcome, ReceiptOutcome, MockResponse
├── state.rs      (110 lines)  - Account/storage state management
└── error.rs      (77 lines)   - Error types

Total: ~1265 lines
```

### Key Data Structures

**Receipt** - a pending function call:
```rust
pub struct Receipt {
    pub id: ReceiptId,
    pub predecessor: AccountId,
    pub receiver: AccountId,
    pub method: String,
    pub args: Vec<u8>,
    pub deposit: NearToken,
    pub gas: NearGas,
    pub depends_on: Vec<ReceiptId>,  // Who we're waiting for
}
```

**Completion** - how a receipt finishes:
```rust
enum Completion {
    Value(Option<Vec<u8>>),  // Returned a value
    Forward(ReceiptId),       // Forwarded to another receipt (promise)
    Failed,                   // Execution failed
}
```

### Action Log Parsing

The VM's `MockedExternal` produces an action log. We parse it to extract new receipts:

```
CreateReceipt { receipt_indices: [0], receiver_id: "b.near" }
  └── receipt_indices are dependencies (positions in action log)
  
FunctionCallWeight { receipt_index: 1, method_name: "foo", args: [...] }
  └── Adds function call to receipt at position 1
```

When `ReturnData::ReceiptIndex(idx)` is returned, it means "forward my result to that receipt."

## What Works

| Feature | Status |
|---------|--------|
| Deploy contracts (WASM bytes or file) | ✅ |
| Function calls with gas tracking | ✅ |
| View calls (read-only) | ✅ |
| Storage persistence | ✅ |
| Block/time advancement | ✅ |
| Mocking external contracts | ✅ |
| Cross-contract calls with callbacks | ✅ |
| Builder pattern API | ✅ |

### Tests (25 passing)

Located in `tests/integration_tests.rs`:
- Basic deploy/call, view calls, storage
- FT contract initialization and metadata
- Cross-contract factorial (recursive calls with callbacks)
- Builder API, gas tracking, mocking

```bash
cargo test  # Runs all 25 tests
```

## What Doesn't Work / Limitations

| Feature | Status | Notes |
|---------|--------|-------|
| `promise_batch` actions | ❌ | Transfer, Deploy, AddKey, etc. not handled |
| Account creation/deletion | ❌ | Not parsed from action log |
| Pure transfers | ❌ | Sending NEAR without function call |
| Storage staking | ❌ | No "need balance to store" enforcement |
| Balance checks | ❌ | Can transfer more than you have |
| Gas refunds | ❌ | Unused gas not refunded |
| 300 TGas limit | ⚠️ | Uses `RuntimeConfigStore::test()` |
| Config customization | ⚠️ | No way to set custom gas limits |

## Mock API

Simplified to one handler per account:

```rust
sim.mock("oracle.near", |method, args| {
    match method {
        "get_price" => MockResponse::success_json(&json!({"price": "5.50"})),
        _ => MockResponse::success(vec![]),
    }
})?;
```

## Example Usage

```rust
use near_contract_sim::ContractSim;

#[test]
fn test_cross_contract() {
    let mut sim = ContractSim::new();
    
    // Deploy contracts
    sim.deploy("a.near", include_bytes!("a.wasm")).unwrap();
    sim.deploy("b.near", include_bytes!("b.wasm")).unwrap();
    
    // Call with cross-contract interaction
    let outcome = sim.call("user.near", "a.near", "call_b", b"{}").unwrap();
    outcome.assert_success();
    
    // Check final result
    let result: String = outcome.json().unwrap();
    assert_eq!(result, "response from b");
}
```
