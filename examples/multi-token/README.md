# Multi Token Example (NEP-245)

This example implements the [NEP-245 Multi Token Standard](https://github.com/near/NEPs/blob/master/neps/nep-0245.md), which supports fungible, semi-fungible, and non-fungible tokens in a single contract.

## Features

- **Core Standard**: Transfer, batch transfer, and transfer-call functionality
- **Approval Management**: Approve other accounts to transfer tokens on your behalf
- **Enumeration**: Query all tokens or tokens owned by a specific account
- **Metadata**: Store token-specific metadata on-chain

## Building

```bash
cd examples/multi-token
cargo build --target wasm32-unknown-unknown --release -p multi-token
cargo build --target wasm32-unknown-unknown --release -p test-mt-receiver
```

The WASM files will be in `target/wasm32-unknown-unknown/release/`.

## Deploying

```bash
# Deploy the contract
near deploy --accountId YOUR_ACCOUNT.testnet \
  --wasmFile target/wasm32-unknown-unknown/release/multi_token.wasm

# Initialize with default metadata
near call YOUR_ACCOUNT.testnet new_default_meta '{"owner_id": "YOUR_ACCOUNT.testnet"}' \
  --accountId YOUR_ACCOUNT.testnet
```

## Usage

### Minting Tokens

Only the contract owner can mint tokens:

```bash
# Mint 1000 fungible-style tokens
near call YOUR_ACCOUNT.testnet mt_mint '{
  "token_id": "gold-coin",
  "token_owner_id": "alice.testnet",
  "amount": "1000",
  "token_metadata": {
    "title": "Gold Coin",
    "description": "In-game currency"
  }
}' --accountId YOUR_ACCOUNT.testnet --deposit 0.1
```

### Checking Balances

```bash
# Single token balance
near view YOUR_ACCOUNT.testnet mt_balance_of '{
  "account_id": "alice.testnet",
  "token_id": "gold-coin"
}'

# Multiple token balances
near view YOUR_ACCOUNT.testnet mt_batch_balance_of '{
  "account_id": "alice.testnet",
  "token_ids": ["gold-coin", "silver-sword"]
}'
```

### Transferring Tokens

```bash
# Simple transfer (requires 1 yoctoNEAR deposit for security)
near call YOUR_ACCOUNT.testnet mt_transfer '{
  "receiver_id": "bob.testnet",
  "token_id": "gold-coin",
  "amount": "100",
  "memo": "Payment for sword"
}' --accountId alice.testnet --depositYocto 1

# Batch transfer
near call YOUR_ACCOUNT.testnet mt_batch_transfer '{
  "receiver_id": "bob.testnet",
  "token_ids": ["gold-coin", "silver-sword"],
  "amounts": ["50", "1"]
}' --accountId alice.testnet --depositYocto 1
```

### Transfer and Call

Transfer tokens to a contract and trigger a callback:

```bash
near call YOUR_ACCOUNT.testnet mt_transfer_call '{
  "receiver_id": "defi-contract.testnet",
  "token_id": "gold-coin",
  "amount": "100",
  "msg": "stake"
}' --accountId alice.testnet --depositYocto 1 --gas 100000000000000
```

### Approval Management

```bash
# Approve bob to spend 500 tokens on your behalf
near call YOUR_ACCOUNT.testnet mt_approve '{
  "token_ids": ["gold-coin"],
  "amounts": ["500"],
  "account_id": "bob.testnet"
}' --accountId alice.testnet --deposit 0.01

# Check if approved
near view YOUR_ACCOUNT.testnet mt_is_approved '{
  "token_ids": ["gold-coin"],
  "approved_account_id": "bob.testnet",
  "amounts": ["500"]
}'

# Revoke approval
near call YOUR_ACCOUNT.testnet mt_revoke '{
  "token_ids": ["gold-coin"],
  "account_id": "bob.testnet"
}' --accountId alice.testnet --depositYocto 1
```

### Burning Tokens

```bash
# Burn your own tokens
near call YOUR_ACCOUNT.testnet mt_burn '{
  "token_id": "gold-coin",
  "amount": "50"
}' --accountId alice.testnet --depositYocto 1
```

### Enumeration

```bash
# List all token types
near view YOUR_ACCOUNT.testnet mt_tokens '{"from_index": "0", "limit": 100}'

# List tokens owned by an account
near view YOUR_ACCOUNT.testnet mt_tokens_for_owner '{
  "account_id": "alice.testnet",
  "from_index": "0",
  "limit": 100
}'

# Get token supply
near view YOUR_ACCOUNT.testnet mt_supply '{"token_id": "gold-coin"}'
```

## Testing

### Unit Tests

```bash
cd examples/multi-token/mt
cargo test
```

### Integration Tests

```bash
cd examples/multi-token
cargo test --test workspaces
```

## Contract Structure

```
multi-token/
├── mt/                          # Main multi-token contract
│   └── src/lib.rs
├── test-token-receiver/         # Test contract for mt_transfer_call
│   └── src/lib.rs
└── tests/
    └── workspaces/              # Integration tests
        ├── test_core.rs
        ├── test_approval.rs
        └── test_enumeration.rs
```

## Key Differences from NFT/FT

| Feature | FT | NFT | Multi Token |
|---------|-----|-----|-------------|
| Token types per contract | 1 | Many (unique) | Many (any amount) |
| Balance per token | Single value | 0 or 1 | 0 to 2^128-1 |
| Batch operations | No | No | Yes |
| Use case | Currencies | Collectibles | Games, mixed assets |

## Standards Compliance

This implementation follows [NEP-245](https://github.com/near/NEPs/blob/master/neps/nep-0245.md) including:

- Core: `mt_transfer`, `mt_batch_transfer`, `mt_transfer_call`, `mt_batch_transfer_call`
- Enumeration: `mt_tokens`, `mt_tokens_for_owner`
- Approval: `mt_approve`, `mt_revoke`, `mt_revoke_all`, `mt_is_approved`
- Events: `mt_mint`, `mt_burn`, `mt_transfer` events
