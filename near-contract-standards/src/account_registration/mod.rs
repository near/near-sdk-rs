use near_sdk::json_types::{ValidAccountId, U128};

pub trait AccountRegistrar {
    /// Registers an account for using smart-contract. Enough NEAR must be attach
    /// to cover storage cost.
    /// Parameters:
    /// * `account_id` - an account to register. Anyone can register any other account.
    /// If the account is already registered then the function early returns and refunds the attached NEAR.
    /// MUST not panic if caller is already registered.
    /// MUST refund the unused attached amount, e.g.
    /// - if the account already registered, it MUST return the full attached amount.
    /// - if the attached account is larger than the `ar_registration_fee` should refund extra.
    /// Returns `false` iff account was already registered.
    /// Panics:
    /// * If not enough deposit was attached to pay for account storage.
    fn ar_register(&mut self, account_id: Option<ValidAccountId>) -> bool;

    /// Checks if the `account_id` is registered.
    fn ar_is_registered(&self, account_id: ValidAccountId) -> bool;

    /// Unregisters the caller for accepting token transfers and return the storage NEAR deposit back.
    /// If the caller is not registered, the function should early return without throwing exception.
    /// If `force=true` the function SHOULD ignore account balances (burn them) and close the account.
    ///     Otherwise, MUST panic if caller has a positive registered balance (eg token holdings)
    /// MUST require exactly 1 yoctoNEAR attached balance to prevent restricted function-call access-key call
    /// (UX wallet security)
    /// Returns `false` if and only if account was not registered.
    fn ar_unregister(&mut self, force: Option<bool>) -> bool;

    /// Returns the amount of NEAR which must be attached for `ar_register` to register an account.
    fn ar_registration_fee(&self) -> U128;
}
