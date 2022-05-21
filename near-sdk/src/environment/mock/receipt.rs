use crate::{AccountId, Balance, Gas, PublicKey};

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Receipt {
    pub receiver_id: AccountId,
    pub actions: Vec<VmAction>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum VmAction {
    CreateAccount,
    DeployContract {
        code: Vec<u8>,
    },
    FunctionCall {
        function_name: String,
        args: Vec<u8>,
        gas: Gas,
        deposit: Balance,
    },
    Transfer {
        deposit: Balance,
    },
    Stake {
        stake: Balance,
        public_key: PublicKey,
    },
    AddKeyWithFullAccess {
        public_key: PublicKey,
        nonce: u64,
    },
    AddKeyWithFunctionCall {
        public_key: PublicKey,
        nonce: u64,
        allowance: Option<Balance>,
        receiver_id: AccountId,
        function_names: Vec<String>,
    },
    DeleteKey {
        public_key: PublicKey,
    },
    DeleteAccount {
        beneficiary_id: AccountId,
    },
}
