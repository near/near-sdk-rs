//! Usability exercise: a NEP-171 NFT with NEP-178 approvals and NEP-181 enumeration,
//! written against `near_sdk::state` (storage v2).
#![cfg(feature = "unstable")]

use std::collections::{BTreeSet, HashMap};

use near_sdk::json_types::U128;
use near_sdk::state::testing::{self, Harness, Receipt};
use near_sdk::state::{GroupedMap, KeyPage, Lazy, Map};
use near_sdk::{AccountId, NearToken, env, near, require};

pub type TokenId = String;

#[near(serializers = [borsh, json])]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct NftContractMetadata {
    pub spec: String,
    pub name: String,
    pub symbol: String,
}

#[near(serializers = [borsh, json])]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct TokenMetadata {
    pub title: Option<String>,
    pub description: Option<String>,
    pub media: Option<String>,
}

/// What `nft_token` returns (NEP-171 `Token` JSON).
#[near(serializers = [json])]
#[derive(Debug, PartialEq)]
pub struct JsonToken {
    pub token_id: TokenId,
    pub owner_id: AccountId,
    pub metadata: Option<TokenMetadata>,
    pub approved_account_ids: HashMap<AccountId, u64>,
}

#[near(storage)]
pub struct Token {
    #[key(1, required)]
    owner_id: AccountId,
    #[key(2)]
    next_approval_id: u64,
    #[key(3)]
    approvals: Map<AccountId, u64>,
    /// Kept out of the token record so transfers do not rewrite it.
    #[key(4)]
    metadata: Lazy<TokenMetadata>,
}

#[near(contract_state(layout = v2))]
pub struct Nft {
    #[key(1, required)]
    owner: AccountId,
    #[key(2)]
    tokens: Map<TokenId, Token>,
    #[key(3)]
    tokens_per_owner: GroupedMap<AccountId, TokenId, ()>,
    #[key(4)]
    metadata: Lazy<NftContractMetadata>,
}

fn assert_one_yocto() {
    require!(env::attached_deposit() == NearToken::from_yoctonear(1), "requires 1 yocto");
}

#[near]
impl Nft {
    #[init]
    /// Revision 3: `Lazy::new` in the struct literal, no `.bind()`: the root's pending `Lazy`
    /// is written by the flush.
    pub fn new(owner: AccountId, metadata: NftContractMetadata) -> Self {
        Self {
            owner,
            tokens: Default::default(),
            tokens_per_owner: Default::default(),
            metadata: Lazy::new(metadata),
        }
    }

    /// Revision 3: `Lazy::new(metadata)` is written when `insert` places the record.
    pub fn nft_mint(&mut self, token_id: TokenId, receiver_id: AccountId, metadata: TokenMetadata) {
        require!(env::predecessor_account_id() == self.owner, "owner only");
        require!(!self.tokens.contains_key(&token_id), "token already exists");
        self.tokens.insert(
            token_id.clone(),
            Token {
                owner_id: receiver_id.clone(),
                next_approval_id: 0,
                approvals: Default::default(),
                metadata: Lazy::new(metadata),
            },
        );
        self.tokens_per_owner.group_mut(&receiver_id).insert(token_id, ());
    }

    #[payable]
    pub fn nft_approve(&mut self, token_id: TokenId, account_id: AccountId) -> u64 {
        assert_one_yocto();
        let token = self.tokens.get_mut(&token_id).expect("no such token");
        require!(token.owner_id == env::predecessor_account_id(), "only the owner can approve");
        let id = token.next_approval_id;
        token.next_approval_id += 1;
        token.approvals.insert(account_id, id);
        id
    }

    #[payable]
    pub fn nft_transfer(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        approval_id: Option<u64>,
        memo: Option<String>,
    ) {
        assert_one_yocto();
        let _ = memo;
        let sender_id = env::predecessor_account_id();
        let token = self.tokens.get_mut(&token_id).expect("no such token");
        let old_owner = token.owner_id.clone();
        if sender_id != old_owner {
            let approved = token.approvals.get(&sender_id).copied();
            require!(approved.is_some(), "sender is not approved");
            if let Some(expected) = approval_id {
                require!(approved == Some(expected), "approval id mismatch");
            }
        }
        require!(old_owner != receiver_id, "the token owner and the receiver should be different");
        token.owner_id = receiver_id.clone();
        token.approvals.clear();
        // `token` (a &mut into `self.tokens`) is still live here in the source order I first
        // wrote; disjoint field borrows make this fine.
        // Revision 3: `remove` on a `()` value skips the entry read, so it costs `discard`.
        self.tokens_per_owner.group_mut(&old_owner).remove(&token_id);
        self.tokens_per_owner.group_mut(&receiver_id).insert(token_id, ());
    }

    pub fn nft_burn(&mut self, token_id: TokenId) {
        let owner = self.tokens.get(&token_id).expect("no such token").owner_id.clone();
        require!(owner == env::predecessor_account_id(), "only the owner can burn");
        self.tokens.remove(&token_id); // cached: no second read
        self.tokens_per_owner.group_mut(&owner).remove(&token_id);
    }

    pub fn nft_token(&self, token_id: TokenId) -> Option<JsonToken> {
        let token = self.tokens.get(&token_id)?;
        Some(JsonToken {
            token_id,
            owner_id: token.owner_id.clone(),
            metadata: token.metadata.get().cloned(),
            approved_account_ids: token.approvals.iter().map(|(k, v)| (k, *v)).collect(),
        })
    }

    // ---- NEP-181 ----

    pub fn nft_total_supply(&self) -> U128 {
        U128(self.tokens.len() as u128)
    }

    pub fn nft_supply_for_owner(&self, account_id: AccountId) -> U128 {
        U128(self.tokens_per_owner.group(&account_id).len() as u128)
    }

    /// NEP-181: offset pagination. Revision 3: `keys().skip(from).take(limit)`; `skip` reads
    /// nothing.
    pub fn nft_tokens_for_owner(
        &self,
        account_id: AccountId,
        from_index: Option<U128>,
        limit: Option<u64>,
    ) -> Vec<JsonToken> {
        let from = usize::try_from(from_index.map_or(0, |i| i.0)).unwrap_or(usize::MAX);
        let limit = usize::try_from(limit.unwrap_or(50)).unwrap_or(usize::MAX);
        self.tokens_per_owner
            .group(&account_id)
            .keys()
            .skip(from)
            .take(limit)
            .map(|id| self.nft_token(id).expect("indexed token exists"))
            .collect()
    }

    /// Cursor variant, which is what the module's `keys_page` is shaped for.
    pub fn nft_token_ids_for_owner(
        &self,
        account_id: AccountId,
        after: Option<TokenId>,
        limit: u32,
    ) -> KeyPage<TokenId> {
        self.tokens_per_owner
            .group(&account_id)
            .keys_page(after.as_ref(), limit)
            .unwrap_or_else(|e| env::panic_str(e.as_ref()))
    }

    pub fn nft_metadata(&self) -> NftContractMetadata {
        self.metadata.get().cloned().unwrap_or_default()
    }
}

// ---------------------------------------------------------------------------------------
// harness
// ---------------------------------------------------------------------------------------

fn id(s: &str) -> AccountId {
    s.parse().unwrap()
}

fn init() -> Receipt {
    Harness::init("minter.near", || {
        Nft::new(
            id("minter.near"),
            NftContractMetadata { spec: "nft-1.0.0".into(), name: "N".into(), symbol: "N".into() },
        )
    })
    .1
}

fn call<R>(who: &str, f: impl FnOnce(&mut Nft) -> R) -> (R, Receipt) {
    Harness::<Nft>::attach().call(who, NearToken::from_yoctonear(1), f)
}

fn view<R>(f: impl FnOnce(&Nft) -> R) -> (R, Receipt) {
    Harness::<Nft>::attach().view(f)
}

fn mint(token: &str, to: &str) {
    call("minter.near", |c| {
        c.nft_mint(token.into(), id(to), TokenMetadata { title: Some(token.into()), ..Default::default() })
    });
}

fn owned_ids(who: &str) -> Vec<TokenId> {
    view(|c| c.nft_tokens_for_owner(id(who), None, Some(1000))).0.into_iter().map(|t| t.token_id).collect()
}

fn keys() -> BTreeSet<Vec<u8>> {
    testing::store().into_keys().collect()
}

// ---------------------------------------------------------------------------------------

#[test]
fn mint_and_read_a_token() {
    init();
    let (_, r) = call("minter.near", |c| {
        c.nft_mint("t1".into(), id("alice.near"), TokenMetadata { title: Some("one".into()), ..Default::default() })
    });
    // has_key(token); token record, metadata, tokens index+slot+header, group entry+index+slot+header
    assert_eq!(r.writes(), 9, "{:#?}", r.ops.iter().map(|o| (o.kind, o.key.len())).collect::<Vec<_>>());
    let (t, r) = view(|c| c.nft_token("t1".into()));
    let t = t.unwrap();
    assert_eq!(t.owner_id, id("alice.near"));
    assert_eq!(t.metadata.unwrap().title.as_deref(), Some("one"));
    assert!(t.approved_account_ids.is_empty());
    assert_eq!(r.reads(), 4, "root, token record, metadata, approvals header");
    assert_eq!(view(|c| c.nft_total_supply()).0, U128(1));
    assert_eq!(view(|c| c.nft_supply_for_owner(id("alice.near"))).0, U128(1));
}

#[test]
#[should_panic(expected = "token already exists")]
fn mint_twice_panics() {
    init();
    mint("t1", "alice.near");
    mint("t1", "bob.near");
}

#[test]
#[should_panic(expected = "owner only")]
fn only_minter_can_mint() {
    init();
    call("alice.near", |c| c.nft_mint("t1".into(), id("alice.near"), Default::default()));
}

#[test]
fn transfer_moves_token_between_owner_lists() {
    init();
    mint("t1", "alice.near");
    mint("t2", "alice.near");
    let (_, r) = call("alice.near", |c| c.nft_transfer(id("bob.near"), "t1".into(), None, None));
    assert_eq!(owned_ids("alice.near"), vec!["t2".to_string()]);
    assert_eq!(owned_ids("bob.near"), vec!["t1".to_string()]);
    assert_eq!(view(|c| c.nft_token("t1".into())).0.unwrap().owner_id, id("bob.near"));
    eprintln!("nft_transfer rwd = {:?}, has_key = {}", r.rwd(), r.has_keys());
    // reads: root, token, approvals header (clear on an empty map), alice index + header +
    // last slot, bob index + header. writes: token, alice moved slot + index + header, bob
    // entry + index + slot + header. removes: alice entry, index, last slot.
    assert_eq!(r.rwd(), (8, 8, 3));
}

#[test]
#[should_panic(expected = "should be different")]
fn transfer_to_self_panics() {
    init();
    mint("t1", "alice.near");
    call("alice.near", |c| c.nft_transfer(id("alice.near"), "t1".into(), None, None));
}

#[test]
fn approved_account_can_transfer_and_approvals_reset() {
    init();
    mint("t1", "alice.near");
    let (aid, _) = call("alice.near", |c| c.nft_approve("t1".into(), id("market.near")));
    assert_eq!(aid, 0);
    call("alice.near", |c| c.nft_approve("t1".into(), id("other.near")));
    let t = view(|c| c.nft_token("t1".into())).0.unwrap();
    assert_eq!(t.approved_account_ids.len(), 2);
    call("market.near", |c| c.nft_transfer(id("carol.near"), "t1".into(), Some(0), None));
    let t = view(|c| c.nft_token("t1".into())).0.unwrap();
    assert_eq!(t.owner_id, id("carol.near"));
    assert!(t.approved_account_ids.is_empty(), "approvals cleared on transfer");
    // next_approval_id keeps counting
    let (aid, _) = call("carol.near", |c| c.nft_approve("t1".into(), id("market.near")));
    assert_eq!(aid, 2);
}

#[test]
#[should_panic(expected = "sender is not approved")]
fn unapproved_transfer_panics() {
    init();
    mint("t1", "alice.near");
    call("mallory.near", |c| c.nft_transfer(id("mallory.near"), "t1".into(), None, None));
}

#[test]
fn burn_with_approvals_leaves_no_keys_behind() {
    init();
    mint("keep", "alice.near"); // so the owner's group survives the burn
    let before = keys();
    mint("t1", "alice.near");
    for a in ["a.near", "b.near", "c.near"] {
        call("alice.near", |c| c.nft_approve("t1".into(), id(a)));
    }
    let (_, r) = call("alice.near", |c| c.nft_burn("t1".into()));
    eprintln!("nft_burn with 3 approvals rwd = {:?}", r.rwd());
    let after = keys();
    let leaked: Vec<_> = after.difference(&before).collect();
    assert!(leaked.is_empty(), "keys left behind by burn: {leaked:?}");
    assert_eq!(after, before);
    assert!(view(|c| c.nft_token("t1".into())).0.is_none());
    assert_eq!(owned_ids("alice.near"), vec!["keep".to_string()]);
}

#[test]
fn burning_the_last_token_empties_the_owner_group() {
    init();
    let before = keys();
    mint("t1", "alice.near");
    call("alice.near", |c| c.nft_approve("t1".into(), id("m.near")));
    call("alice.near", |c| c.nft_burn("t1".into()));
    assert_eq!(keys(), before);
}

#[test]
#[should_panic(expected = "only the owner can burn")]
fn only_owner_can_burn() {
    init();
    mint("t1", "alice.near");
    call("bob.near", |c| c.nft_burn("t1".into()));
}

#[test]
fn tokens_for_owner_paginates_and_stops_at_the_end() {
    init();
    for i in 0..7 {
        mint(&format!("t{i}"), "alice.near");
    }
    let page = |from: u128, limit: u64| {
        view(|c| c.nft_tokens_for_owner(id("alice.near"), Some(U128(from)), Some(limit)))
            .0
            .into_iter()
            .map(|t| t.token_id)
            .collect::<Vec<_>>()
    };
    assert_eq!(page(0, 3), vec!["t0", "t1", "t2"]);
    assert_eq!(page(3, 3), vec!["t3", "t4", "t5"]);
    assert_eq!(page(6, 3), vec!["t6"]);
    assert!(page(7, 3).is_empty(), "past the end is empty, not a panic");
    assert!(page(1_000_000, 3).is_empty());
    assert!(page(u128::MAX, 3).is_empty());
    assert!(view(|c| c.nft_tokens_for_owner(id("nobody.near"), None, None)).0.is_empty());
}

#[test]
fn cursor_pagination_over_owner_ids() {
    init();
    for i in 0..5 {
        mint(&format!("t{i}"), "alice.near");
    }
    let (p1, _) = view(|c| c.nft_token_ids_for_owner(id("alice.near"), None, 2));
    assert_eq!(p1.keys, vec!["t0", "t1"]);
    let (p2, _) = view(|c| c.nft_token_ids_for_owner(id("alice.near"), p1.next.clone(), 2));
    assert_eq!(p2.keys, vec!["t2", "t3"]);
    let (p3, _) = view(|c| c.nft_token_ids_for_owner(id("alice.near"), p2.next.clone(), 2));
    assert_eq!(p3.keys, vec!["t4"]);
    assert_eq!(p3.next, None);
}

/// The cursor is a key: if that token moves away between pages, the next page panics.
#[test]
#[should_panic(expected = "page cursor key no longer exists")]
fn cursor_page_after_the_cursor_token_is_transferred_panics() {
    init();
    for i in 0..5 {
        mint(&format!("t{i}"), "alice.near");
    }
    let (p1, _) = view(|c| c.nft_token_ids_for_owner(id("alice.near"), None, 2));
    call("alice.near", |c| c.nft_transfer(id("bob.near"), "t1".into(), None, None));
    view(|c| c.nft_token_ids_for_owner(id("alice.near"), p1.next.clone(), 2));
}

/// Swap-remove: moving a token out of the middle of an owner's list puts the last token in
/// its place, so enumeration order is not mint order after a transfer.
#[test]
fn transfer_reorders_the_owner_list() {
    init();
    for i in 0..4 {
        mint(&format!("t{i}"), "alice.near");
    }
    call("alice.near", |c| c.nft_transfer(id("bob.near"), "t1".into(), None, None));
    assert_eq!(owned_ids("alice.near"), vec!["t0", "t3", "t2"]);
}

/// Guess: a `Lazy` inside a record cannot be filled before the record is placed. Building
/// the token with its metadata in one expression is the first thing I tried to write.
#[test]
#[should_panic(expected = "not bound")]
fn setting_a_nested_lazy_before_insert_panics() {
    init();
    call("minter.near", |c| {
        let mut t = Token {
            owner_id: id("alice.near"),
            next_approval_id: 0,
            approvals: Default::default(),
            metadata: Default::default(),
        };
        t.metadata.set(TokenMetadata::default());
        c.tokens.insert("t1".into(), t);
    });
}

/// Revision 3: `Lazy::new` is the struct-literal form (`nft_mint` uses it); `set` on an
/// unplaced `Lazy` still panics, as above. Here: the root's `Lazy::new` from `#[init]`, with
/// no `.bind()`, is written by the flush.
#[test]
fn init_lazy_new_at_the_root_is_written_without_bind() {
    init();
    let (m, _) = view(|c| c.nft_metadata());
    assert_eq!(m.name, "N");
    let (t, _) = view(|c| c.nft_token("none".into()));
    assert!(t.is_none());
    mint("t1", "alice.near");
    let (t, _) = view(|c| c.nft_token("t1".into()));
    assert_eq!(t.unwrap().metadata.unwrap().title.as_deref(), Some("t1"));
}

/// Revision 3: `skip(from)` reads nothing, so a deep offset page costs what the first does.
#[test]
fn offset_page_cost_does_not_depend_on_the_offset() {
    init();
    for i in 0..6 {
        mint(&format!("t{i}"), "alice.near");
    }
    let (first, r0) = view(|c| c.nft_tokens_for_owner(id("alice.near"), Some(U128(0)), Some(1)));
    let (deep, r5) = view(|c| c.nft_tokens_for_owner(id("alice.near"), Some(U128(5)), Some(1)));
    assert_eq!((first[0].token_id.as_str(), deep[0].token_id.as_str()), ("t0", "t5"));
    assert_eq!(r0.rwd(), r5.rwd());
}
