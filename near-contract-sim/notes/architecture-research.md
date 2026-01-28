# Architecture Research: NEAR Testing Primitives

## near-workspaces Architecture

### Process Model

```
┌─────────────────────────────────────────────────────────────────┐
│  Your Test Code (Rust)                                          │
│  ├── near-workspaces crate                                      │
│  │   ├── near-jsonrpc-client  ──────► JSON-RPC HTTP requests    │
│  │   └── near-sandbox         ──────► Downloads/spawns binary   │
└─────────────────────────────────────────────────────────────────┘
                          │
                          ▼  (HTTP RPC)
┌─────────────────────────────────────────────────────────────────┐
│  neard-sandbox binary (separate process)                        │
│  ├── Full NEAR node in "sandbox mode"                           │
│  ├── Consensus disabled (single-node, instant blocks)           │
│  ├── Exposes JSON-RPC on localhost port                         │
│  └── Uses nearcore with real runtime                            │
└─────────────────────────────────────────────────────────────────┘
```

### What The Sandbox Actually Is

The sandbox is a **full NEAR node binary** (`neard`) compiled with special sandbox features:
- **Full runtime**: Uses real `near-vm-runner` for WASM execution
- **Single-node consensus**: No validators, blocks produced on demand
- **Genesis from scratch**: Fresh chain state each run
- **RPC interface**: Full JSON-RPC API (same as mainnet/testnet)
- **Not a "lite version"**: It's the full `nearcore` codebase

### What Makes It Slow

1. **Process Startup** (~1-3 seconds per test suite)
   - Spawning `neard-sandbox` process
   - Initializing genesis state
   - Opening RocksDB
   - Starting RPC server

2. **RPC Overhead** (per call)
   - HTTP request/response serialization
   - JSON-RPC encoding/decoding
   - Network stack overhead (even localhost)

3. **Block Production**
   - Each transaction goes through full block production
   - Receipt execution follows real protocol

4. **Binary Download** (first run only)
   - Downloads ~100MB+ binary on first use

### Existing Speed Options

- `NEAR_RPC_TIMEOUT_SECS` - Reduce RPC timeout
- `NEAR_SANDBOX_BIN_PATH` - Use pre-existing binary (skip download)
- `worker.fast_forward(delta)` - Skip blocks for time-sensitive tests

**No in-process mode exists.**

---

## nearcore Runtime Layering

### Crate Hierarchy

```
+------------------------------------------------------------------+
|                         nearcore                                  |
|  (Full node: consensus, networking, RPC, block production)       |
+------------------------------------------------------------------+
                              │
                              ▼
+------------------------------------------------------------------+
|                      node-runtime                                 |
|  - Receipt processing (action, data, promise yield/resume)       |
|  - Action execution (transfer, stake, deploy, function_call)     |
|  - State transitions via TrieUpdate                              |
|  - Validator accounting, gas refunds, congestion control         |
|  - Uses near-store for state persistence                         |
|  ⚠️  NOT PUBLISHED TO CRATES.IO (internal only)                  |
+------------------------------------------------------------------+
                              │
                              ▼
+------------------------------------------------------------------+
|                      near-vm-runner                               |
|  - WASM contract compilation & execution                         |
|  - VMLogic - the host functions interface                        |
|  - Gas metering, memory management                               |
|  - Multiple backends: Wasmtime, near-vm (custom)                 |
|  ✅ Published to crates.io                                       |
+------------------------------------------------------------------+
                              │
                              ▼
+------------------------------------------------------------------+
|                     near-vm (optional)                            |
|  - NEAR's custom WASM compiler (singlepass)                      |
|  - Alternative to Wasmtime                                       |
+------------------------------------------------------------------+
```

### Where Key Operations Happen

| Operation | Handled By |
|-----------|------------|
| Receipt processing | `node-runtime` |
| Action execution (CreateAccount, Transfer, Stake, AddKey, etc.) | `node-runtime` |
| Function calls (WASM execution) | `near-vm-runner` (invoked by node-runtime) |
| State transitions | `node-runtime` + `near-store` |
| Gas metering | `near-vm-runner` (WASM), `node-runtime` (action fees) |
| Promise/callback handling | `node-runtime` (receipt scheduling) |

### The Problem

`node-runtime` has all the logic we need, but it's:
- **Not published** (`publish = false` in Cargo.toml)
- **Tightly coupled** to nearcore's storage layer (Trie)
- **Not designed for external use**

So we're stuck with `near-vm-runner`, which only does WASM execution - not the full runtime logic.

---

## Comparison Table

| Aspect | near-workspaces | near-contract-sim | testing_env! |
|--------|----------------|-------------------|--------------|
| **Speed** | Slow (process + RPC) | Fast (in-process) | Very Fast |
| **Cross-contract** | Yes (real) | Yes (real WASM) | No (logs only) |
| **WASM execution** | Yes | Yes | No (native) |
| **Gas metering** | Yes (full) | Yes (partial) | Limited |
| **Mocking** | No | Yes | Manual |
| **Protocol fidelity** | High | Medium | Low |
| **Dependencies** | neard binary (~100MB) | near-vm-runner crate | None |
| **Transfers** | Yes | No | No |
| **Account creation** | Yes | No | No |
| **Storage staking** | Yes | No | No |

---

## Alternative Approaches We Didn't Pursue

### 1. In-Process Sandbox
Could the sandbox logic run in-process instead of as a subprocess?

**Challenge:** nearcore is designed as a node, not a library. Significant refactoring would be needed.

### 2. Interface-Level Mocking
Instead of running real WASM, mock the contract interface:

```rust
#[near_contract_interface]
trait FungibleToken {
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128);
    fn ft_balance_of(&self, account_id: AccountId) -> U128;
}

let mock_ft = MockContract::<FungibleToken>::new();
mock_ft.when(|c| c.ft_balance_of(any())).returns(U128(1000));
```

**Challenge:** Doesn't test real contract code. Good for testing callers, not callees.

### 3. Record/Replay
Record real workspaces interactions, replay them fast. Like VCR for HTTP tests.

**Challenge:** Tests become stale when contracts change. Good for regression, not development.

### 4. Faster Workspaces
Instead of building an alternative, make workspaces faster:
- Persistent sandbox with state reset
- Memory-backed storage (tmpfs)
- Connection pooling
- Batch transaction submission

**This might be the most pragmatic path.** Improve the official tool rather than building a parallel one.

---

## Key Insight

The fundamental issue is that **NEAR's testing infrastructure wasn't designed with "fast in-process testing" as a goal**. The primitives are:

1. `testing_env!` - Very fast, but no cross-contract
2. `near-workspaces` - Full fidelity, but slow

There's no middle ground by design. Creating one means either:
- Reimplementing runtime logic (what we did, incompletely)
- Getting nearcore to expose better primitives (upstream work)
- Making workspaces faster (pragmatic improvement)

Option 3 is probably the best ROI if the goal is "help NEAR developers test faster."
