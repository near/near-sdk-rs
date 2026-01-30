# NEP-245 Multi Token Implementation Specification

## Overview

This document specifies the implementation of NEP-245 (Multi Token Standard) for `near-contract-standards`. The implementation follows patterns established by the existing NFT (NEP-171) and FT (NEP-141) implementations, prioritizing developer experience, safety, and idiomatic Rust.

## Design Principles

1. **Follow NFT patterns closely** — Same structure, same DX, familiar to existing developers
2. **No deprecated macros** — Recommend manual trait implementations (macros are deprecated in NFT/FT)
3. **Hardened events** — Prevent log overflow bugs discovered in production
4. **Clean, idiomatic API** — Ergonomic builder patterns, sensible defaults
5. **Production-ready** — Include approval management and storage management

## File Structure

```
near-contract-standards/src/multi_token/
├── mod.rs                    # Public exports
├── token.rs                  # Token + TokenId types
├── core/
│   ├── mod.rs               # MultiTokenCore trait + ext_contract
│   ├── core_impl.rs         # MultiToken struct + implementations [NEW]
│   ├── receiver.rs          # MultiTokenReceiver trait
│   └── resolver.rs          # MultiTokenResolver trait
├── approval/                 # [NEW MODULE]
│   ├── mod.rs               # MultiTokenApproval trait
│   └── approval_impl.rs     # Approval implementation
├── enumeration/              # [RESTRUCTURE]
│   ├── mod.rs               # Trait definition (moved from enumeration.rs)
│   └── enumeration_impl.rs  # Implementation [NEW]
├── metadata.rs              # Metadata structs (exists, minor updates)
├── events.rs                # Events with safety checks (rewrite)
├── storage_impl.rs          # StorageManagement implementation [NEW]
└── utils.rs                 # Helper functions [NEW]
```

## Core Components

### 1. Token Types (`token.rs`)

```rust
pub type TokenId = String;

/// Approval information for a specific grant
#[derive(Clone, Debug, PartialEq, Eq)]
#[near(serializers = [borsh, json])]
pub struct Approval {
    pub approval_id: u64,
    pub amount: u128,
}

/// The Token struct returned by view methods
#[derive(Debug, Clone, PartialEq, Eq)]
#[near(serializers = [json])]
pub struct Token {
    pub token_id: TokenId,
    pub owner_id: Option<AccountId>,  // None for fungible-style tokens
    pub metadata: Option<MTTokenMetadata>,
    pub approved_account_ids: Option<HashMap<AccountId, Approval>>,
}
```

### 2. MultiToken Struct (`core/core_impl.rs`)

The main implementation struct storing all state:

```rust
#[near]
pub struct MultiToken {
    /// Contract owner (can mint new tokens)
    pub owner_id: AccountId,
    
    /// Storage cost tracking
    pub extra_storage_in_bytes_per_token: StorageUsage,
    
    /// TokenId -> creator/owner (for NFT-like tokens with supply=1)
    pub owner_by_id: TreeMap<TokenId, AccountId>,
    
    /// TokenId -> total supply
    pub total_supply: LookupMap<TokenId, u128>,
    
    /// TokenId -> (AccountId -> balance)
    pub balances: LookupMap<TokenId, LookupMap<AccountId, u128>>,
    
    /// Optional: Metadata extension
    pub token_metadata_by_id: Option<LookupMap<TokenId, MTTokenMetadata>>,
    
    /// Optional: Enumeration extension  
    pub tokens_per_owner: Option<LookupMap<AccountId, UnorderedSet<TokenId>>>,
    
    /// Optional: Approval extension
    /// TokenId -> (GranterAccountId -> (GranteeAccountId -> Approval))
    pub approvals_by_id: Option<LookupMap<TokenId, HashMap<AccountId, HashMap<AccountId, Approval>>>>,
    pub next_approval_id_by_id: Option<LookupMap<TokenId, u64>>,
}
```

#### Constructor

```rust
impl MultiToken {
    pub fn new<O, T, E, A>(
        owner_by_id_prefix: O,
        owner_id: AccountId,
        token_metadata_prefix: Option<T>,
        enumeration_prefix: Option<E>,
        approval_prefix: Option<A>,
    ) -> Self
    where
        O: IntoStorageKey,
        T: IntoStorageKey,
        E: IntoStorageKey,
        A: IntoStorageKey;
}
```

#### Internal Methods

```rust
impl MultiToken {
    // Token existence
    pub fn token_exists(&self, token_id: &TokenId) -> bool;
    
    // Balance operations
    pub fn internal_balance_of(&self, account_id: &AccountId, token_id: &TokenId) -> u128;
    pub fn internal_unwrap_balance_of(&self, account_id: &AccountId, token_id: &TokenId) -> u128;
    
    // Mint/Burn
    pub fn internal_mint(
        &mut self,
        token_id: TokenId,
        token_owner_id: AccountId,
        amount: u128,
        token_metadata: Option<MTTokenMetadata>,
    ) -> Token;
    
    pub fn internal_burn(
        &mut self,
        token_id: &TokenId,
        account_id: &AccountId,
        amount: u128,
        memo: Option<String>,
    );
    
    // Transfer
    pub fn internal_transfer(
        &mut self,
        sender_id: &AccountId,
        receiver_id: &AccountId,
        token_id: &TokenId,
        amount: u128,
        approval_id: Option<u64>,
        memo: Option<String>,
    ) -> (AccountId, Option<HashMap<AccountId, Approval>>);
    
    pub fn internal_transfer_call(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        token_id: TokenId,
        amount: U128,
        approval_id: Option<u64>,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>>;
}
```

### 3. Trait Implementations

The `MultiToken` struct implements:

- `MultiTokenCore` — transfer, batch_transfer, transfer_call, etc.
- `MultiTokenResolver` — mt_resolve_transfer
- `MultiTokenApproval` — mt_approve, mt_revoke, mt_is_approved
- `MultiTokenEnumeration` — mt_tokens, mt_tokens_for_owner
- `StorageManagement` — storage_deposit, storage_withdraw, etc.

### 4. Events (`events.rs`)

Events with builder pattern and safety checks:

```rust
#[must_use = "make sure to `.emit()` this event"]
pub struct MtMint<'a> {
    pub owner_id: &'a AccountIdRef,
    pub token_ids: &'a [&'a str],
    pub amounts: &'a [U128],
    pub memo: Option<&'a str>,
}

impl<'a> MtMint<'a> {
    /// Create a new mint event
    pub fn new(owner_id: &'a AccountIdRef, token_ids: &'a [&'a str], amounts: &'a [U128]) -> Self;
    
    /// Add an optional memo
    pub fn memo(self, memo: &'a str) -> Self;
    
    /// Emit the event
    pub fn emit(self);
    
    /// Emit multiple events
    pub fn emit_many(data: &[MtMint<'_>]);
}

// Similar for MtTransfer and MtBurn
```

### 5. Approval Management (`approval/`)

```rust
pub trait MultiTokenApproval {
    /// Approve account to transfer tokens on behalf of owner
    fn mt_approve(
        &mut self,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        account_id: AccountId,
        msg: Option<String>,
    ) -> Option<Promise>;
    
    /// Revoke approval for specific tokens
    fn mt_revoke(&mut self, token_ids: Vec<TokenId>, account_id: AccountId);
    
    /// Revoke all approvals for specific tokens
    fn mt_revoke_all(&mut self, token_ids: Vec<TokenId>);
    
    /// Check if account is approved
    fn mt_is_approved(
        &self,
        token_ids: Vec<TokenId>,
        approved_account_id: AccountId,
        amounts: Vec<U128>,
        approval_ids: Option<Vec<u64>>,
    ) -> bool;
}
```

### 6. Storage Management (`storage_impl.rs`)

```rust
impl StorageManagement for MultiToken {
    fn storage_deposit(
        &mut self,
        account_id: Option<AccountId>,
        registration_only: Option<bool>,
    ) -> StorageBalance;
    
    fn storage_withdraw(&mut self, amount: Option<U128>) -> StorageBalance;
    
    fn storage_unregister(&mut self, force: Option<bool>) -> bool;
    
    fn storage_balance_bounds(&self) -> StorageBalanceBounds;
    
    fn storage_balance_of(&self, account_id: AccountId) -> Option<StorageBalance>;
}
```

### 7. Utility Functions (`utils.rs`)

```rust
/// Calculate storage bytes for an approval
pub fn bytes_for_approved_account_id(account_id: &AccountId) -> u64;

/// Refund storage deposit when approvals are cleared
pub fn refund_approved_account_ids(
    account_id: AccountId,
    approved_account_ids: &HashMap<AccountId, Approval>,
) -> Promise;

/// Refund excess deposit after storage is paid
pub fn refund_deposit(storage_used: u64);
pub fn refund_deposit_to_account(storage_used: u64, account_id: AccountId);

/// Assert at least 1 yoctoNEAR attached
pub fn assert_at_least_one_yocto();

/// Validate batch operation arguments
pub fn assert_valid_batch_args(token_ids: &[TokenId], amounts: &[U128]);
pub fn assert_valid_batch_approvals(
    token_ids: &[TokenId], 
    approvals: &Option<Vec<Option<(AccountId, u64)>>>
);
```

## Example Usage

```rust
use near_contract_standards::multi_token::{
    MultiToken, Token, TokenId,
    core::MultiTokenCore,
    approval::MultiTokenApproval,
    enumeration::MultiTokenEnumeration,
    metadata::{MTContractMetadata, MTTokenMetadata, MultiTokenMetadataProvider},
};
use near_sdk::collections::LazyOption;
use near_sdk::{near, AccountId, BorshStorageKey, PanicOnDefault};

#[derive(BorshStorageKey)]
#[near]
enum StorageKey {
    MultiToken,
    Metadata,
    TokenMetadata,
    Enumeration,
    Approval,
}

#[derive(PanicOnDefault)]
#[near(contract_state)]
pub struct Contract {
    tokens: MultiToken,
    metadata: LazyOption<MTContractMetadata>,
}

#[near]
impl Contract {
    #[init]
    pub fn new(owner_id: AccountId, metadata: MTContractMetadata) -> Self {
        Self {
            tokens: MultiToken::new(
                StorageKey::MultiToken,
                owner_id,
                Some(StorageKey::TokenMetadata),
                Some(StorageKey::Enumeration),
                Some(StorageKey::Approval),
            ),
            metadata: LazyOption::new(StorageKey::Metadata, Some(&metadata)),
        }
    }

    #[payable]
    pub fn mt_mint(
        &mut self,
        token_id: TokenId,
        token_owner_id: AccountId,
        amount: U128,
        token_metadata: Option<MTTokenMetadata>,
    ) -> Token {
        // Only owner can mint
        assert_eq!(env::predecessor_account_id(), self.tokens.owner_id);
        self.tokens.internal_mint(token_id, token_owner_id, amount.0, token_metadata)
    }
}

// Implement traits by delegating to self.tokens
#[near]
impl MultiTokenCore for Contract {
    #[payable]
    fn mt_transfer(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        amount: U128,
        approval: Option<(AccountId, u64)>,
        memo: Option<String>,
    ) {
        self.tokens.mt_transfer(receiver_id, token_id, amount, approval, memo);
    }
    
    // ... other methods delegate similarly
}
```

## Implementation Phases

### Phase 1: Core Infrastructure
- [x] Token types (`token.rs`)
- [ ] `MultiToken` struct with basic storage
- [ ] `internal_mint`, `internal_burn`, `internal_transfer`
- [ ] `MultiTokenCore` implementation
- [ ] `MultiTokenResolver` implementation

### Phase 2: Events
- [ ] Rewrite events with builder pattern
- [ ] Add safety checks (log length validation)
- [ ] Update existing code to use new events

### Phase 3: Enumeration
- [ ] Restructure into `enumeration/` directory
- [ ] Implement `MultiTokenEnumeration` for `MultiToken`

### Phase 4: Approval Management
- [ ] Create `approval/` module
- [ ] Implement `MultiTokenApproval` trait
- [ ] Implement approval receiver (`mt_on_approve`)

### Phase 5: Storage Management
- [ ] Implement `StorageManagement` for `MultiToken`
- [ ] Add account registration tracking

### Phase 6: Utilities
- [ ] Port helpers from NFT
- [ ] Add MT-specific batch validation helpers

### Phase 7: Example & Tests
- [ ] Create example contract
- [ ] Create test receiver contract
- [ ] Write integration tests

## Estimated Line Counts

| Component | Lines |
|-----------|-------|
| `core/core_impl.rs` | ~550 |
| `approval/` | ~210 |
| `enumeration/enumeration_impl.rs` | ~80 |
| `events.rs` (rewrite) | ~280 |
| `storage_impl.rs` | ~120 |
| `utils.rs` | ~80 |
| Updates to existing files | ~100 |
| **Library Total** | **~1,420** |
| Example contract | ~200 |
| Test receiver contract | ~80 |
| Integration tests | ~400 |
| **Grand Total** | **~2,100** |

## References

- [NEP-245 Specification](https://github.com/near/NEPs/blob/master/neps/nep-0245.md)
- [NEP-245 Approval Management](https://github.com/near/NEPs/blob/master/neps/nep-0245/ApprovalManagement.md)
- [NEP-245 Enumeration](https://github.com/near/NEPs/blob/master/neps/nep-0245/Enumeration.md)
- [NEP-245 Metadata](https://github.com/near/NEPs/blob/master/neps/nep-0245/Metadata.md)
- [near/intents NEP-245 implementation](https://github.com/near/intents/tree/main/nep245)
- [Existing NFT implementation](../../non_fungible_token/)
- [Existing FT implementation](../../fungible_token/)
