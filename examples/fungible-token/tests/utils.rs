extern crate fungible_token;

use near_crypto::{InMemorySigner, KeyType, Signer};
use near_primitives::{
    account::AccessKey,
    hash::CryptoHash,
    transaction::{ExecutionOutcome, ExecutionStatus, Transaction},
    types::{AccountId, Balance},
};
use near_runtime_standalone::{init_runtime_and_signer, RuntimeStandalone};
use near_sdk::json_types::U128;
use serde::de::DeserializeOwned;

use serde_json::json;

pub fn call_view<I: ToString, O: DeserializeOwned>(
    runtime: &mut RuntimeStandalone,
    account_id: &AccountId,
    method: &str,
    args: I,
) -> O {
    let args = args.to_string();
    let result = runtime
        .view_method_call(account_id, method, args.as_bytes())
        .unwrap()
        .0;
    let output: O = serde_json::from_reader(result.as_slice()).unwrap();
    output
}

pub fn call_token<I: ToString, O: DeserializeOwned>(
    runtime: &mut RuntimeStandalone,
    method: &str,
    args: I,
) -> O {
    call_view(runtime, &TOKEN_ACCOUNT_ID.into(), method, args)
}

pub const TOKEN_ACCOUNT_ID: &str = "token";

pub fn ntoy(near_amount: Balance) -> Balance {
    near_amount * 10u128.pow(24)
}

lazy_static::lazy_static! {
    static ref TOKEN_WASM_BYTES: &'static [u8] = include_bytes!("../res/fungible_token.wasm").as_ref();
}

pub struct ExternalUser {
    account_id: AccountId,
    signer: InMemorySigner,
}

impl ExternalUser {
    pub fn new(account_id: AccountId, signer: InMemorySigner) -> Self {
        Self { account_id, signer }
    }

    pub fn init(&self, runtime: &mut RuntimeStandalone, total_supply: Balance) -> ExecutionOutcome {
        let args = json!({
            "owner_id": self.account_id,
            "total_supply": format!("{}", total_supply)
        })
        .to_string()
        .as_bytes()
        .to_vec();

        let tx = self
            .new_tx(runtime, TOKEN_ACCOUNT_ID.into())
            .create_account()
            .transfer(ntoy(100))
            .deploy_contract(TOKEN_WASM_BYTES.to_vec())
            .function_call("new".into(), args, 1000000000000, 0)
            .sign(&self.signer);
        let res = runtime.resolve_tx(tx).unwrap();
        assert!(matches!(res, ExecutionOutcome { status: ExecutionStatus::SuccessValue(_), ..}));
        // match res.status {
        //     ExecutionStatus::SuccessValue(_) => (),
        //     ExecutionStatus::Failure(_) => (),
        //     x=>panic!(x)
        // }
        // assert!(matches!(res, ExecutionOutcome { status: ExecutionStatus::SuccessValue(_), ..}));
        runtime.process_all().unwrap();
        res
    }

    pub fn account_id(&self) -> &AccountId {
        &self.account_id
    }

    pub fn signer(&self) -> &InMemorySigner {
        &self.signer
    }

    pub fn create_external(
        &self,
        runtime: &mut RuntimeStandalone,
        new_account_id: AccountId,
        amount: Balance,
    ) -> ExternalUser {
        let new_signer =
            InMemorySigner::from_seed(&new_account_id, KeyType::ED25519, &new_account_id);
        let tx = self
            .new_tx(runtime, new_account_id.clone())
            .create_account()
            .add_key(new_signer.public_key(), AccessKey::full_access())
            .transfer(amount)
            .sign(&self.signer);
        let res = runtime.resolve_tx(tx).unwrap();
        // assert!(matches!(res, ExecutionOutcome { status: ExecutionStatus::SuccessValue(_), ..}));
        runtime.process_all().unwrap();
        ExternalUser {
            account_id: new_account_id,
            signer: new_signer,
        }
    }

    fn new_tx(&self, runtime: &RuntimeStandalone, receiver_id: AccountId) -> Transaction {
        let nonce = runtime
            .view_access_key(&self.account_id, &self.signer.public_key())
            .unwrap()
            .nonce
            + 1;
        Transaction::new(
            self.account_id.clone(),
            self.signer.public_key(),
            receiver_id,
            nonce,
            CryptoHash::default(),
        )
    }

    pub fn transfer(
        &self,
        runtime: &mut RuntimeStandalone,
        new_owner_id: AccountId,
        amount: u128,
    ) -> ExecutionOutcome {
        let args = json!({
        "new_owner_id": new_owner_id,
        "amount": format!("{}", amount)
        })
        .to_string()
        .as_bytes()
        .to_vec();
        let tx = self
            .new_tx(runtime, TOKEN_ACCOUNT_ID.into())
            .function_call("transfer".into(), args, 10000000000000000, 0)
            .sign(&self.signer);
        let res = runtime.resolve_tx(tx).unwrap();
        // assert!(matches!(res, ExecutionOutcome { status: ExecutionStatus::SuccessValue(_), ..}));
        runtime.process_all().unwrap();
        res
    }

    pub fn get_total_supply(&self, runtime: &mut RuntimeStandalone) -> Balance {
        let balance = runtime
            .view_method_call(
                &TOKEN_ACCOUNT_ID.into(),
                "get_total_supply",
                json!({}).to_string().as_bytes(),
            )
            .unwrap()
            .0;
        u128::from(serde_json::from_slice::<U128>(balance.as_slice()).unwrap())
        // u128::from(serde_json::from_slice::<U128>(balance.as_slice()).unwrap())
    }

    pub fn get_balance(&self, runtime: &mut RuntimeStandalone, account_id: AccountId) -> Balance {
        let balance = runtime
            .view_method_call(
                &TOKEN_ACCOUNT_ID.into(),
                "get_balance",
                json!({ "owner_id": account_id }).to_string().as_bytes(),
            )
            .unwrap()
            .0;
        u128::from(serde_json::from_slice::<U128>(balance.as_slice()).unwrap())
        // u128::from(serde_json::from_slice::<U128>(balance.as_slice()).unwrap())
    }
}

pub fn init_contract(total_supply: Balance) -> (RuntimeStandalone, ExternalUser) {
    let (mut runtime, signer) = init_runtime_and_signer(&"root".into());
    let root = ExternalUser::new("root".into(), signer);
    // root.create_external(&mut runtime, root.account_id().into(), total_supply / 2);
    root.init(&mut runtime, total_supply);
    return (runtime, root);
}
