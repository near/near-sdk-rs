//! Usability exercise: a NEP-141 fungible token with NEP-145 storage management, written
//! against `near_sdk::state` (storage v2) the way an ordinary contract would be.
#![cfg(feature = "unstable")]

use near_sdk::json_types::U128;
use near_sdk::state::testing::{self, Harness, Receipt};
use near_sdk::state::{ContractState, Lazy, LookupMap};
use near_sdk::{AccountId, NearToken, Promise, env, near, require};

/// NEP-145 bond per registered account. A real contract measures `env::storage_usage()`
/// around a dummy registration in `new`; v2 keys are longer than v1's so the v1 constant
/// (125 bytes) is not obviously right. See the `storage_bond_covers_one_account` test.
const ACCOUNT_STORAGE_BYTES: u128 = 200;

fn storage_bond() -> NearToken {
    env::storage_byte_cost().saturating_mul(ACCOUNT_STORAGE_BYTES)
}

#[near(serializers = [borsh, json])]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct FtMetadata {
    pub spec: String,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
}

#[near(serializers = [json])]
#[derive(Debug, PartialEq)]
pub struct StorageBalance {
    pub total: U128,
    pub available: U128,
}

/// One record per registered account: balance and the storage bond paid for it.
#[near(storage)]
pub struct FtAccount {
    #[key(1)]
    balance: u128,
    #[key(2)]
    storage_deposit: u128,
}

#[near(contract_state(layout = v2))]
pub struct Ft {
    #[key(1, required)]
    owner: AccountId,
    #[key(2)]
    total_supply: u128,
    #[key(3)]
    accounts: LookupMap<AccountId, FtAccount>,
    #[key(4)]
    metadata: Lazy<FtMetadata>,
}

#[near]
impl Ft {
    #[init]
    pub fn new(owner: AccountId, total_supply: U128, metadata: FtMetadata) -> Self {
        let mut this = Self {
            owner: owner.clone(),
            total_supply: total_supply.0,
            accounts: Default::default(),
            metadata: Default::default(),
        }
        .bind();
        this.accounts.insert(
            owner,
            FtAccount { balance: total_supply.0, storage_deposit: storage_bond().as_yoctonear() },
        );
        this.metadata.set(metadata);
        this
    }

    // ---- NEP-145 ----

    #[payable]
    pub fn storage_deposit(&mut self, account_id: Option<AccountId>) -> StorageBalance {
        let account_id = account_id.unwrap_or_else(env::predecessor_account_id);
        let deposit = env::attached_deposit();
        if self.accounts.contains_key(&account_id) {
            // Already registered: refund everything (NEP-145 says so for fixed-size bonds).
            if !deposit.is_zero() {
                Promise::new(env::predecessor_account_id()).transfer(deposit).detach();
            }
        } else {
            let bond = storage_bond();
            require!(deposit >= bond, "attached deposit is less than the storage bond");
            self.accounts
                .insert(account_id.clone(), FtAccount { balance: 0, storage_deposit: bond.as_yoctonear() });
            let refund = deposit.saturating_sub(bond);
            if !refund.is_zero() {
                Promise::new(env::predecessor_account_id()).transfer(refund).detach();
            }
        }
        self.storage_balance_of(account_id).expect("just registered")
    }

    /// Refunds the bond and removes the account. With a nonzero balance it panics unless
    /// `force`, which burns the balance.
    #[payable]
    pub fn storage_unregister(&mut self, force: Option<bool>) -> bool {
        require!(env::attached_deposit() == NearToken::from_yoctonear(1), "requires 1 yocto");
        let id = env::predecessor_account_id();
        let Some(acct) = self.accounts.get(&id) else {
            return false;
        };
        if acct.balance != 0 && !force.unwrap_or(false) {
            env::panic_str("cannot unregister with a positive balance without force");
        }
        let removed = self.accounts.remove(&id).expect("checked above");
        self.total_supply -= removed.balance;
        Promise::new(id).transfer(NearToken::from_yoctonear(removed.storage_deposit + 1)).detach();
        true
    }

    pub fn storage_balance_of(&self, account_id: AccountId) -> Option<StorageBalance> {
        self.accounts
            .get(&account_id)
            .map(|a| StorageBalance { total: U128(a.storage_deposit), available: U128(0) })
    }

    // ---- NEP-141 ----

    #[payable]
    pub fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>) {
        require!(env::attached_deposit() == NearToken::from_yoctonear(1), "requires 1 yocto");
        let sender_id = env::predecessor_account_id();
        require!(sender_id != receiver_id, "sender and receiver must differ");
        require!(amount.0 > 0, "amount must be positive");
        let _ = memo;
        // Check the receiver first so a failed transfer touches nothing.
        require!(self.accounts.contains_key(&receiver_id), "receiver is not registered");
        let sender = self.accounts.get_mut(&sender_id).expect("sender is not registered");
        sender.balance = sender.balance.checked_sub(amount.0).expect("insufficient balance");
        let receiver = self.accounts.get_mut(&receiver_id).expect("receiver is not registered");
        receiver.balance = receiver.balance.checked_add(amount.0).expect("balance overflow");
    }

    pub fn ft_balance_of(&self, account_id: AccountId) -> U128 {
        U128(self.accounts.get(&account_id).map_or(0, |a| a.balance))
    }

    pub fn ft_total_supply(&self) -> U128 {
        U128(self.total_supply)
    }

    pub fn ft_metadata(&self) -> FtMetadata {
        self.metadata.get().cloned().unwrap_or_default()
    }
}

// ---------------------------------------------------------------------------------------
// harness (revision 3: `state::testing::Harness`)
// ---------------------------------------------------------------------------------------

fn id(s: &str) -> AccountId {
    s.parse().unwrap()
}

fn meta() -> FtMetadata {
    FtMetadata { spec: "ft-1.0.0".into(), name: "Token".into(), symbol: "TKN".into(), decimals: 24 }
}

fn init() -> Receipt {
    Harness::init("owner.near", || Ft::new(id("owner.near"), U128(1_000), meta())).1
}

fn call_with<R>(who: &str, deposit: NearToken, f: impl FnOnce(&mut Ft) -> R) -> (R, Receipt) {
    Harness::<Ft>::attach().call(who, deposit, f)
}

fn call<R>(who: &str, f: impl FnOnce(&mut Ft) -> R) -> (R, Receipt) {
    call_with(who, NearToken::from_yoctonear(1), f)
}

fn view<R>(f: impl FnOnce(&Ft) -> R) -> (R, Receipt) {
    Harness::<Ft>::attach().view(f)
}

fn register(who: &str) {
    call_with(who, storage_bond(), |c| c.storage_deposit(None));
}

fn balance(who: &str) -> u128 {
    view(|c| c.ft_balance_of(id(who))).0.0
}

// ---------------------------------------------------------------------------------------

#[test]
fn init_sets_supply_owner_balance_and_metadata() {
    let r = init();
    assert_eq!(r.rwd(), (0, 3, 0), "owner account record, metadata, root");
    assert_eq!(view(|c| c.ft_total_supply()).0, U128(1_000));
    assert_eq!(balance("owner.near"), 1_000);
    assert_eq!(view(|c| c.ft_metadata()).0, meta());
}

#[test]
fn storage_deposit_registers_once_and_refunds_the_rest() {
    init();
    let (sb, r) = call_with("alice.near", storage_bond().saturating_mul(3), |c| c.storage_deposit(None));
    assert_eq!(sb.total.0, storage_bond().as_yoctonear());
    // root read, has_key on alice; account record write. Root unchanged, so not written.
    assert_eq!(r.rwd(), (1, 1, 0));
    assert_eq!(r.has_keys(), 1);
    // second deposit: no state change
    let (_, r) = call_with("alice.near", storage_bond(), |c| c.storage_deposit(None));
    assert_eq!(r.writes(), 0);
    assert_eq!(balance("alice.near"), 0);
}

#[test]
#[should_panic(expected = "less than the storage bond")]
fn storage_deposit_too_small_panics() {
    init();
    call_with("alice.near", NearToken::from_yoctonear(1), |c| c.storage_deposit(None));
}

#[test]
fn storage_deposit_for_another_account() {
    init();
    call_with("alice.near", storage_bond(), |c| c.storage_deposit(Some(id("bob.near"))));
    assert!(view(|c| c.storage_balance_of(id("bob.near"))).0.is_some());
    assert!(view(|c| c.storage_balance_of(id("alice.near"))).0.is_none());
}

#[test]
fn ft_transfer_moves_balance_between_two_accounts() {
    init();
    register("alice.near");
    let (_, r) = call("owner.near", |c| c.ft_transfer(id("alice.near"), U128(300), None));
    assert_eq!(balance("owner.near"), 700);
    assert_eq!(balance("alice.near"), 300);
    // root read, has_key(receiver), sender read, receiver read; two record writes.
    assert_eq!(r.rwd(), (3, 2, 0), "{:?}", r.ops);
    assert_eq!(r.has_keys(), 1, "the receiver check is a has_key, and does not save the later read");
}

#[test]
#[should_panic(expected = "sender and receiver must differ")]
fn ft_transfer_to_self_panics() {
    init();
    call("owner.near", |c| c.ft_transfer(id("owner.near"), U128(1), None));
}

#[test]
#[should_panic(expected = "insufficient balance")]
fn ft_transfer_more_than_balance_panics() {
    init();
    register("alice.near");
    call("alice.near", |c| c.ft_transfer(id("owner.near"), U128(1), None));
}

#[test]
#[should_panic(expected = "receiver is not registered")]
fn ft_transfer_to_unregistered_panics() {
    init();
    call("owner.near", |c| c.ft_transfer(id("nobody.near"), U128(1), None));
}

/// A panic mid-method aborts the receipt on chain, so nothing is written. In the harness the
/// panic unwinds out of `call` before `state_write`; the next `begin_call` drops the cache.
#[test]
fn failed_transfer_leaves_state_untouched() {
    init();
    register("alice.near");
    let res = std::panic::catch_unwind(|| {
        call("alice.near", |c| c.ft_transfer(id("owner.near"), U128(5), None));
    });
    assert!(res.is_err());
    assert_eq!(balance("owner.near"), 1_000);
    assert_eq!(balance("alice.near"), 0);
}

#[test]
fn unregister_with_zero_balance_removes_the_account() {
    init();
    register("alice.near");
    let (ok, r) = call("alice.near", |c| c.storage_unregister(None));
    assert!(ok);
    assert_eq!(r.rwd(), (2, 0, 1), "root + record read; record remove");
    assert!(view(|c| c.storage_balance_of(id("alice.near"))).0.is_none());
    let (again, _) = call("alice.near", |c| c.storage_unregister(None));
    assert!(!again);
}

#[test]
#[should_panic(expected = "positive balance without force")]
fn unregister_with_nonzero_balance_panics() {
    init();
    register("alice.near");
    call("owner.near", |c| c.ft_transfer(id("alice.near"), U128(10), None));
    call("alice.near", |c| c.storage_unregister(None));
}

#[test]
fn force_unregister_burns_the_balance() {
    init();
    register("alice.near");
    call("owner.near", |c| c.ft_transfer(id("alice.near"), U128(10), None));
    let (ok, r) = call("alice.near", |c| c.storage_unregister(Some(true)));
    assert!(ok);
    assert_eq!(r.rwd(), (2, 1, 1), "root rewritten for total_supply, record removed");
    assert_eq!(view(|c| c.ft_total_supply()).0, U128(990));
    assert_eq!(balance("alice.near"), 0);
    // Re-registering starts from zero, not from a stale record.
    register("alice.near");
    assert_eq!(balance("alice.near"), 0);
}

#[test]
fn views_read_only_what_they_need() {
    init();
    let (_, r) = view(|c| c.ft_total_supply());
    assert_eq!(r.rwd(), (1, 0, 0), "root only; metadata is Lazy");
    let (_, r) = view(|c| c.ft_balance_of(id("owner.near")));
    assert_eq!(r.rwd(), (2, 0, 0));
}

/// NEP-145 bond sizing: how many bytes does one registration really use? Measured through
/// the mocked blockchain's `storage_usage`, which the counting mock does not track.
#[test]
fn storage_bond_covers_one_account() {
    testing::set_context("owner.near", NearToken::from_yoctonear(0));
    testing::use_env_host();
    let c = Ft::new(id("owner.near"), U128(1_000), meta());
    c.state_write();
    drop(c);
    // testing_env! resets storage_usage from the builder, so measure after switching context.
    testing::set_context("a-rather-long-account-name-for-measuring-storage.near", storage_bond());
    let before = env::storage_usage();
    testing::begin_call();
    let mut c = Ft::state_read().unwrap();
    c.storage_deposit(None);
    c.state_write();
    drop(c);
    let used = env::storage_usage() - before;
    eprintln!("one registration: {used} bytes");
    assert!(
        (used as u128) <= ACCOUNT_STORAGE_BYTES,
        "one registration used {used} bytes, bond covers {ACCOUNT_STORAGE_BYTES}"
    );
}

// ---------------------------------------------------------------------------------------
// D5: `storage_usage()` inside a v2 mutating method is an error; `storage_delta()` is the
// post-flush value
// ---------------------------------------------------------------------------------------

/// The NEP-145 before/after pattern inside a v2 method: the "after" read panics, pointing at
/// `storage_delta`, instead of returning the unchanged pre-flush value.
#[test]
#[should_panic(expected = "Use the post-flush `storage_delta()` instead")]
fn storage_usage_inside_a_v2_mutating_method_panics() {
    init();
    call_with("alice.near", storage_bond(), |c| {
        let before = env::storage_usage(); // nothing written yet: allowed
        c.storage_deposit(None);
        env::storage_usage() - before // after a write, before the flush: error
    });
}

/// `Receipt::storage_delta()` (from a harness call, so after the flush) equals what the
/// mocked blockchain's `storage_usage` says the same registration used, and a removal gives
/// it back.
#[test]
fn storage_delta_is_the_post_flush_storage_change() {
    init();
    let (_, r) = call_with("alice.near", storage_bond(), |c| c.storage_deposit(None));
    let mock_delta = r.storage_delta();
    assert!(mock_delta > 0);

    // the same call through the mocked blockchain, measured with storage_usage between calls
    testing::set_context("owner.near", NearToken::from_yoctonear(0));
    testing::use_env_host();
    let c = Ft::new(id("owner.near"), U128(1_000), meta());
    c.state_write();
    drop(c);
    testing::set_context("alice.near", storage_bond());
    let before = env::storage_usage();
    testing::env_log_start();
    testing::begin_call();
    let mut c = Ft::state_read().unwrap();
    c.storage_deposit(None);
    c.state_write();
    drop(c);
    let used = env::storage_usage() as i64 - before as i64;
    let logged = testing::env_log_take().storage_delta();
    testing::env_log_stop();
    assert_eq!(used, mock_delta, "mock receipt agrees with the mocked blockchain");
    assert_eq!(logged, used, "env op log agrees too");

    // unregistering returns the bytes
    testing::set_context("alice.near", NearToken::from_yoctonear(1));
    testing::env_log_start();
    testing::begin_call();
    let mut c = Ft::state_read().unwrap();
    c.storage_unregister(None);
    c.state_write();
    drop(c);
    let freed = testing::env_log_take().storage_delta();
    testing::env_log_stop();
    assert_eq!(freed, -used);
}
