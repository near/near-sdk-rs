use near_account_id::AccountId;
use near_sdk_macros::near;

use crate::CryptoHash;

#[near(inside_nearsdk)]
#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq)]
pub enum AccountContract {
    None,
    Local(CryptoHash),
    Global(CryptoHash),
    GlobalByAccount(AccountId),
}
