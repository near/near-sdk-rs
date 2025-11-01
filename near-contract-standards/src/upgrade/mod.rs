use near_sdk::json_types::U64;
use near_sdk::{env, near, require, AccountId, Duration, Promise, Timestamp};

type WrappedDuration = U64;

pub trait Ownable {
    fn assert_owner(&self) {
        require!(env::predecessor_account_id() == self.get_owner(), "Owner must be predecessor");
    }
    fn get_owner(&self) -> AccountId;
    fn set_owner(&mut self, owner: AccountId);
}

pub trait Upgradable {
    fn get_staging_duration(&self) -> WrappedDuration;
    fn stage_code(&mut self, code: Vec<u8>, timestamp: Timestamp);
    fn deploy_code(&mut self) -> Promise;

    /// Implement migration for the next version.
    /// Should be `unimplemented` for a new contract.
    /// TODO: consider adding version of the contract stored in the storage?
    fn migrate(&mut self) {
        unimplemented!();
    }
}

#[near]
pub struct Upgrade {
    pub owner: AccountId,
    pub staging_duration: Duration,
    pub staging_timestamp: Timestamp,
}

impl Upgrade {
    pub fn new(owner: AccountId, staging_duration: Duration) -> Self {
        Self { owner, staging_duration, staging_timestamp: 0 }
    }
}

impl Ownable for Upgrade {
    fn get_owner(&self) -> AccountId {
        self.owner.clone()
    }

    fn set_owner(&mut self, owner: AccountId) {
        self.assert_owner();
        self.owner = owner;
    }
}

impl Upgradable for Upgrade {
    fn get_staging_duration(&self) -> WrappedDuration {
        self.staging_duration.into()
    }

    fn stage_code(&mut self, code: Vec<u8>, timestamp: Timestamp) {
        self.assert_owner();
        require!(
            env::block_timestamp() + self.staging_duration < timestamp,
            "Timestamp must be later than staging duration"
        );
        // Writes directly into storage to avoid serialization penalty by using default struct.
        env::storage_write(b"upgrade", &code);
        self.staging_timestamp = timestamp;
    }

    fn deploy_code(&mut self) -> Promise {
        if self.staging_timestamp < env::block_timestamp() {
            env::panic_str(
                format!(
                    "Deploy code too early: staging ends on {}",
                    self.staging_timestamp + self.staging_duration
                )
                .as_str(),
            );
        }
        let code = env::storage_read(b"upgrade")
            .unwrap_or_else(|| env::panic_str("No upgrade code available"));
        env::storage_remove(b"upgrade");
        Promise::new(env::current_account_id()).deploy_contract(code)
    }
}
