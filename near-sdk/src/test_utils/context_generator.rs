use crate::{
    AccountId,
    Balance,
    BlockHeight,
    EpochHeight,
    Gas,
    PublicKey,
    StorageUsage,
    test_utils::{VMContextBuilder},
};

use std::convert::{
    TryFrom,
    TryInto,
};

fn to_bool(bytes: &[u8]) -> bool {
    let (int_bytes, _) = bytes.split_at(std::mem::size_of::<bool>());
    // If  the byte == 1 return true, else false
    let byte = u8::from_le_bytes(int_bytes.try_into().unwrap());
    assert!(byte<=1, "Invalid type for boolean, byte value is ({})", byte);
    byte == 1
}

//
// Most of the traits below are just copy and paste to allow the 
// macro to convert the args to similar types of data.
//

/// Trait used for implementing macro context_generator!
/// Not meant to be implemented manually.
pub trait VMCBAuto{
    fn to_account_id(&self) -> Result<AccountId, String>;

    fn to_u64(&self) -> Result<u64, String>;

    fn to_u128(&self) -> Result<u128, String>;

    fn to_vec_u8(&self) -> Result<Vec<u8>, String>;

    fn to_bool(&self) -> Result<bool, String>;
}



impl VMCBAuto for usize{
    fn to_u64(&self) -> Result<u64, String> { Ok(*self as u64) }

    fn to_account_id(&self) -> Result<AccountId, String> { Err(String::from("Invalid type for conversion in Macro")) }

    fn to_u128(&self) -> Result<u128, String> { Ok(*self as u128) }

    fn to_vec_u8(&self) -> Result<Vec<u8>, String> { Err(String::from("Invalid type for conversion in Macro")) }

    fn to_bool(&self) -> Result<bool, String> { Ok(*self != 0) }
}


impl VMCBAuto for i32{
    fn to_u64(&self) -> Result<u64, String> { Ok(*self as u64) }

    fn to_account_id(&self) -> Result<AccountId, String> { Err(String::from("Invalid type for conversion in Macro")) }

    fn to_u128(&self) -> Result<u128, String> { Ok(*self as u128) }

    fn to_vec_u8(&self) -> Result<Vec<u8>, String> { Err(String::from("Invalid type for conversion in Macro")) }

    fn to_bool(&self) -> Result<bool, String> { Ok(*self != 0) }
}


impl VMCBAuto for i64{
    fn to_u64(&self) -> Result<u64, String> { Ok(*self as u64) }

    fn to_account_id(&self) -> Result<AccountId, String> { Err(String::from("Invalid type for conversion in Macro")) }

    fn to_u128(&self) -> Result<u128, String> { Ok(*self as u128) }

    fn to_vec_u8(&self) -> Result<Vec<u8>, String> { Err(String::from("Invalid type for conversion in Macro")) }

    fn to_bool(&self) -> Result<bool, String> { Ok(*self != 0) }
}


impl VMCBAuto for u32{
    fn to_u64(&self) -> Result<u64, String> { Ok(*self as u64) }

    fn to_account_id(&self) -> Result<AccountId, String> { Err(String::from("Invalid type for conversion in Macro")) }

    fn to_u128(&self) -> Result<u128, String> { Ok(*self as u128) }

    fn to_vec_u8(&self) -> Result<Vec<u8>, String> { Err(String::from("Invalid type for conversion in Macro")) }

    fn to_bool(&self) -> Result<bool, String> { Ok(*self != 0) }
}


impl VMCBAuto for u64{
    fn to_u64(&self) -> Result<u64, String> {
        Ok(self.clone())
    }

    fn to_account_id(&self) -> Result<AccountId, String> { Err(String::from("Invalid type for conversion in Macro")) }

    fn to_u128(&self) -> Result<u128, String> { Ok(*self as u128) }

    fn to_vec_u8(&self) -> Result<Vec<u8>, String> { Err(String::from("Invalid type for conversion in Macro")) }

    fn to_bool(&self) -> Result<bool, String> { Err(String::from("Invalid type for conversion in Macro")) }
}


impl VMCBAuto for u128{
    fn to_u128(&self) -> Result<u128, String> {
        Ok(self.clone())
    }

    fn to_account_id(&self) -> Result<AccountId, String> { Err(String::from("Invalid type for conversion in Macro")) }

    fn to_u64(&self) -> Result<u64, String> { Err(String::from("Invalid type for conversion in Macro")) }

    fn to_vec_u8(&self) -> Result<Vec<u8>, String> { Err(String::from("Invalid type for conversion in Macro")) }
    
    fn to_bool(&self) -> Result<bool, String> { Err(String::from("Invalid type for conversion in Macro")) }
}

impl VMCBAuto for Vec<u8>{
    fn to_vec_u8(&self) -> Result<Vec<u8>, String> {
        Ok(self.clone())
    }

    fn to_account_id(&self) -> Result<AccountId, String> { Err(String::from("Invalid type for conversion in Macro")) }

    fn to_u64(&self) -> Result<u64, String> { Err(String::from("Invalid type for conversion in Macro")) }

    fn to_u128(&self) -> Result<u128, String> { Err(String::from("Invalid type for conversion in Macro")) }

    fn to_bool(&self) -> Result<bool, String> { Err(String::from("Invalid type for conversion in Macro")) }
}

impl VMCBAuto for &AccountId {
    fn to_account_id(&self) -> Result<AccountId, String> {
        Ok((*self).clone())
    }

    fn to_u64(&self) -> Result<u64, String> { Err(String::from("Invalid type for conversion in Macro")) }

    fn to_u128(&self) -> Result<u128, String> { Err(String::from("Invalid type for conversion in Macro")) }

    fn to_vec_u8(&self) -> Result<Vec<u8>, String> { Err(String::from("Invalid type for conversion in Macro")) }

    fn to_bool(&self) -> Result<bool, String> { Err(String::from("Invalid type for conversion in Macro")) }
}

impl VMCBAuto for AccountId {
    fn to_account_id(&self) -> Result<AccountId, String> {
        Ok(self.clone())
    }

    fn to_u64(&self) -> Result<u64, String> { Err(String::from("Invalid type for conversion in Macro")) }

    fn to_u128(&self) -> Result<u128, String> { Err(String::from("Invalid type for conversion in Macro")) }

    fn to_vec_u8(&self) -> Result<Vec<u8>, String> { Err(String::from("Invalid type for conversion in Macro")) }

    fn to_bool(&self) -> Result<bool, String> { Err(String::from("Invalid type for conversion in Macro")) }
}

impl VMCBAuto for String {
    fn to_account_id(&self) -> Result<AccountId, String> {
        
        match AccountId::try_from(self.clone()) {
            Ok(value) => Ok(value),
            Err(err) => {
                Err(format!("{}", err))
            }
        }
    }

    fn to_u64(&self) -> Result<u64, String> { Err(String::from("Invalid type for conversion in Macro")) }

    fn to_u128(&self) -> Result<u128, String> { Err(String::from("Invalid type for conversion in Macro")) }

    fn to_vec_u8(&self) -> Result<Vec<u8>, String> { Err(String::from("Invalid type for conversion in Macro")) }

    fn to_bool(&self) -> Result<bool, String> { Err(String::from("Invalid type for conversion in Macro")) }
}


impl VMCBAuto for str{
    fn to_account_id(&self) -> Result<AccountId, String> {
        
        match AccountId::try_from(String::from(self)){
            Ok(value) => Ok(value),
            Err(err) => {
                Err(format!("{}", err))
            }
        }
    }

    fn to_u64(&self) -> Result<u64, String> { Err(String::from("Invalid type for conversion in Macro")) }

    fn to_u128(&self) -> Result<u128, String> { Err(String::from("Invalid type for conversion in Macro")) }

    fn to_vec_u8(&self) -> Result<Vec<u8>, String> { Err(String::from("Invalid type for conversion in Macro")) }

    fn to_bool(&self) -> Result<bool, String> { Err(String::from("Invalid type for conversion in Macro")) }
}


impl VMCBAuto for &str{
    fn to_account_id(&self) -> Result<AccountId, String> {
        match AccountId::try_from(String::from(*self)){
            Ok(value) => Ok(value),
            Err(err) => {
                Err(format!("{}", err))
            }
        }
    }

    fn to_u64(&self) -> Result<u64, String> { Err(String::from("Invalid type for conversion in Macro")) }

    fn to_u128(&self) -> Result<u128, String> { Err(String::from("Invalid type for conversion in Macro")) }

    fn to_vec_u8(&self) -> Result<Vec<u8>, String> { Err(String::from("Invalid type for conversion in Macro")) }

    fn to_bool(&self) -> Result<bool, String> { Err(String::from("Invalid type for conversion in Macro")) }
}


impl VMCBAuto for bool{
    fn to_account_id(&self) -> Result<AccountId, String> { Err(String::from("Invalid type for conversion in Macro")) }

    fn to_u64(&self) -> Result<u64, String> { Err(String::from("Invalid type for conversion in Macro")) }

    fn to_u128(&self) -> Result<u128, String> { Err(String::from("Invalid type for conversion in Macro")) }

    fn to_vec_u8(&self) -> Result<Vec<u8>, String> { Err(String::from("Invalid type for conversion in Macro")) }

    fn to_bool(&self) -> Result<bool, String> { 
        Ok(to_bool(& Vec::from([self.clone().into()])))
    }
}

/// Used by macro context_generator, not meant to be implemented manually.
pub enum VMCB {
    CurrentAccountId(AccountId),
    SignerAccountId(AccountId),
    SignerAccountPK(PublicKey),
    PredecessorAccountId(AccountId),
    BlockIndex(BlockHeight),
    BlockTimestamp(u64),
    EpochHeight(EpochHeight),
    AccountBalance(Balance),
    AccountLockedBalance(Balance),
    StorageUsage(StorageUsage),
    AttachedDeposit(Balance),
    PrepaidGas(Gas),
    RandomSeed(Vec<u8>),
    IsView(bool),
}

impl VMCB{
    pub fn new<D: VMCBAuto> (name: &str, arg: D) -> Result<VMCB, String>{
        let response = match name{
            "current_account_id" => {               VMCB::CurrentAccountId(arg.to_account_id()?) },
            "signer_account_id" => {                VMCB::SignerAccountId(arg.to_account_id()?) },
            "signer_account_pk" => {                VMCB::SignerAccountPK(PublicKey::try_from(arg.to_vec_u8()?).unwrap()) },
            "predecessor_account_id" => {           VMCB::PredecessorAccountId(arg.to_account_id()?) },
            "block_index" | "block_height" => {     VMCB::BlockIndex(arg.to_u64()?)},
            "block_timestamp" => {                  VMCB::BlockTimestamp(arg.to_u64()?)},
            "epoch_height" => {                     VMCB::EpochHeight(arg.to_u64()?) },
            "account_balance" => {                  VMCB::AccountBalance(arg.to_u128()?) },
            "account_locked_balance" => {           VMCB::AccountLockedBalance(arg.to_u128()?) },
            "storage_usage" => {                    VMCB::StorageUsage(arg.to_u64()?)},
            "attached_deposit" => {                 VMCB::AttachedDeposit(arg.to_u128()?) },
            "prepaid_gas" => {                      VMCB::PrepaidGas(Gas::from(arg.to_u64()?)) },
            "random_seed" => {                      VMCB::RandomSeed(arg.to_vec_u8()?) },
            "is_view" => {                          VMCB::IsView(arg.to_bool()?) },
            other => {
                return Err(format!("Invalid string arg for VMCB {}", other));
            }
        };
        Ok(response)
    }

    // Consume self to do the action in the builder
    pub fn action(self, builder: &mut VMContextBuilder){
        match self{
            VMCB::CurrentAccountId(value) => { builder.current_account_id(value); },
            VMCB::SignerAccountId(value) => { builder.signer_account_id(value); },
            VMCB::SignerAccountPK(value) => { builder.signer_account_pk(value); },
            VMCB::PredecessorAccountId(value) => { builder.predecessor_account_id(value);},
            VMCB::BlockIndex(value) => { builder.block_index(value); },
            VMCB::BlockTimestamp(value) => { builder.block_timestamp(value); },
            VMCB::EpochHeight(value) => { builder.epoch_height(value); },
            VMCB::AccountBalance(value) => { builder.account_balance(value); },
            VMCB::AccountLockedBalance(value) => { builder.account_locked_balance(value); },
            VMCB::StorageUsage(value) => { builder.storage_usage(value);},
            VMCB::AttachedDeposit(value) => { builder.attached_deposit(value); },
            VMCB::PrepaidGas(value) => { builder.prepaid_gas(value);},
            VMCB::RandomSeed(value) => {builder.random_seed((value).try_into().unwrap());},
            VMCB::IsView(value) => { builder.is_view(value);},
        }
    }
}


/// A macro for generating the boilerplate for the virtual machine context.
/// 
/// In summary, this macro creates an instance of VMContextBuilder, assigns attributes given,
/// then call testing_env! with given builder.
/// 
/// Only accepts pairs of arguments. The first argument of each pair is a string literal for the attribute,
/// for each string literal (on the left), it will call VMContextBuilder method (on the right):
/// 
///  "current_account_id" => VMContextBuilder::current_account_id
///  "signer_account_id" => VMContextBuilder::signer_account
///  "signer_account_pk" => VMContextBuilder::signer_account_pk
///  "predecessor_account_id" => VMContextBuilder::predecessor_account_id
///  "block_index" | "block_height" => VMContextBuilder::block_index
///  "block_timestamp" => VMContextBuilder::block_timestamp
///  "epoch_height" => VMContextBuilder::epoch_height
///  "account_balance" => VMContextBuilder::account_balance
///  "account_locked_balance" => VMContextBuilder::account_locked_balance
///  "storage_usage" => VMContextBuilder::storage_usage
///  "attached_deposit" => VMContextBuilder::attached_deposit
///  "prepaid_gas" => VMContextBuilder::prepaid_gas
///  "random_seed" => VMContextBuilder::random_seed
///  "is_view" => VMContextBuilder::is_view
/// 
/// Attribute args (on the right) doesn't need to be exactly what VMContext Builder need.
/// If we give it a positive i32 and it expects a u64, the macro will convert it automatically.
/// If method expects a String, using str, &str, and String will all work as well. But having
/// a String when it expected a u64 will cause a panic because the chances of being intentional
/// is slim. Any expression is valid for Attribute args.
/// 
/// #[test]
/// fn context_generator_tests(){
///     use near_sdk::{
///         AccountId,
///         env,
///         context_generator,
///     };
///
///     let contributor_account_id = AccountId::try_from(String::from("bob")).unwrap();
///
///     fn a_timestamp() -> u64 {
///         1234567891011
///     }
///
///
///     context_generator!(
///         "attached_deposit", 100000000000000000 as u64,
///         "predecessor_account_id", &contributor_account_id,
///         "signer_account_id", contributor_account_id.clone(),
///         "block_index", 3000000,
///         "block_timestamp", a_timestamp(),
///     );
///
///     assert_eq!(env::attached_deposit(), 100000000000000000 as u128);
///     assert_eq!(env::predecessor_account_id(), contributor_account_id);
///     assert_eq!(env::signer_account_id(), contributor_account_id);
///     assert_eq!(env::block_height(), 3000000);
///     assert_eq!(env::block_timestamp(), 1234567891011);
/// }
/// 
/// 
#[macro_export]
macro_rules! context_generator {
    (@partial_setup () $(,)?) => {};

    (@partial_setup $builder: ident $(,)?) => {};

    (@partial_setup $builder: ident, $invalid: expr $(,)?) => {
        compile_error!("Only allow pairs of arguments.");
    };

    (@partial_setup $builder: ident, $name: expr, $arg: expr, $($others: tt)*) => {
        println!("Setting context attribute '{}' with the value '{}'", $name, $arg);
        match $crate::test_utils::context_generator::VMCB::new(&$name, $arg){
            Ok(value) => { 
                value.action(&mut $builder); 
            },
            Err(err) => { 
                panic!("{}", err);
            },
        };
        

        $crate::context_generator!(@partial_setup $builder, $($others)*);
    };

    () => {};

    ($name: expr, $arg: expr, $($others: tt)*) => {
        let mut builder = near_sdk::test_utils::VMContextBuilder::new();
        
        $crate::context_generator!(@partial_setup builder, $name, $arg, $($others)*);

        near_sdk::testing_env!(builder.build());
    };
}

// #[allow(unused_imports)]
// pub use context_generator_local as context_generator;
