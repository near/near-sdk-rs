use near_primitives_core::types::GasWeight;
use near_vm_runner::logic::mocks::mock_external::MockAction as LogicMockAction;
use near_vm_runner::logic::types::ReceiptIndex;

use crate::{AccountId, Gas, NearToken};

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Receipt {
    pub receiver_id: AccountId,
    pub receipt_indices: Vec<ReceiptIndex>,
    pub actions: Vec<MockAction>,
}

#[derive(serde::Serialize)]
#[serde(remote = "GasWeight")]
struct GasWeightSer(u64);

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub enum MockAction {
    CreateReceipt {
        receipt_indices: Vec<ReceiptIndex>,
        receiver_id: AccountId,
    },
    CreateAccount {
        receipt_index: ReceiptIndex,
    },
    DeployContract {
        receipt_index: ReceiptIndex,
        code: Vec<u8>,
    },
    FunctionCallWeight {
        receipt_index: ReceiptIndex,
        method_name: Vec<u8>,
        args: Vec<u8>,
        attached_deposit: NearToken,
        prepaid_gas: Gas,
        #[serde(with = "GasWeightSer")]
        gas_weight: GasWeight,
    },
    Transfer {
        receipt_index: ReceiptIndex,
        deposit: NearToken,
    },
    Stake {
        receipt_index: ReceiptIndex,
        stake: NearToken,
        public_key: near_crypto::PublicKey,
    },
    DeleteAccount {
        receipt_index: ReceiptIndex,
        beneficiary_id: AccountId,
    },
    DeleteKey {
        receipt_index: ReceiptIndex,
        public_key: near_crypto::PublicKey,
    },
    AddKeyWithFunctionCall {
        receipt_index: ReceiptIndex,
        public_key: near_crypto::PublicKey,
        nonce: u64,
        allowance: Option<NearToken>,
        receiver_id: AccountId,
        method_names: Vec<String>,
    },
    AddKeyWithFullAccess {
        receipt_index: ReceiptIndex,
        public_key: near_crypto::PublicKey,
        nonce: u64,
    },
    YieldCreate {
        data_id: near_primitives::hash::CryptoHash,
        receiver_id: AccountId,
    },
    YieldResume {
        data: Vec<u8>,
        data_id: near_primitives::hash::CryptoHash,
    },
}

impl MockAction {
    pub fn receipt_index(&self) -> Option<ReceiptIndex> {
        match self {
            MockAction::CreateReceipt { .. } => None,
            MockAction::CreateAccount { receipt_index } => Some(*receipt_index),
            MockAction::DeployContract { receipt_index, .. } => Some(*receipt_index),
            MockAction::FunctionCallWeight { receipt_index, .. } => Some(*receipt_index),
            MockAction::Transfer { receipt_index, .. } => Some(*receipt_index),
            MockAction::Stake { receipt_index, .. } => Some(*receipt_index),
            MockAction::DeleteAccount { receipt_index, .. } => Some(*receipt_index),
            MockAction::DeleteKey { receipt_index, .. } => Some(*receipt_index),
            MockAction::AddKeyWithFunctionCall { receipt_index, .. } => Some(*receipt_index),
            MockAction::AddKeyWithFullAccess { receipt_index, .. } => Some(*receipt_index),
            MockAction::YieldCreate { .. } => None,
            MockAction::YieldResume { .. } => None,
        }
    }
}

fn map_vec_str(vec_str: Vec<Vec<u8>>) -> Vec<String> {
    vec_str
        .into_iter()
        .map(|element| {
            let string: String = String::from_utf8(element).unwrap();
            string
        })
        .collect()
}

impl From<LogicMockAction> for MockAction {
    fn from(value: LogicMockAction) -> Self {
        match value {
            LogicMockAction::CreateReceipt { receipt_indices, receiver_id } => {
                Self::CreateReceipt { receipt_indices, receiver_id }
            }
            LogicMockAction::CreateAccount { receipt_index } => {
                Self::CreateAccount { receipt_index }
            }
            LogicMockAction::DeployContract { receipt_index, code } => {
                Self::DeployContract { receipt_index, code }
            }
            LogicMockAction::FunctionCallWeight {
                receipt_index,
                method_name,
                args,
                attached_deposit,
                prepaid_gas,
                gas_weight,
            } => Self::FunctionCallWeight {
                receipt_index,
                method_name,
                args,
                attached_deposit: NearToken::from_yoctonear(attached_deposit),
                prepaid_gas: Gas::from_gas(prepaid_gas),
                gas_weight,
            },
            LogicMockAction::Transfer { receipt_index, deposit } => {
                MockAction::Transfer { receipt_index, deposit: NearToken::from_yoctonear(deposit) }
            }
            LogicMockAction::Stake { receipt_index, stake, public_key } => MockAction::Stake {
                receipt_index,
                stake: NearToken::from_yoctonear(stake),
                public_key,
            },
            LogicMockAction::DeleteAccount { receipt_index, beneficiary_id } => {
                Self::DeleteAccount { receipt_index, beneficiary_id }
            }
            LogicMockAction::DeleteKey { receipt_index, public_key } => {
                Self::DeleteKey { receipt_index, public_key }
            }
            LogicMockAction::AddKeyWithFunctionCall {
                receipt_index,
                public_key,
                nonce,
                allowance,
                receiver_id,
                method_names,
            } => Self::AddKeyWithFunctionCall {
                receipt_index,
                public_key,
                nonce,
                allowance: allowance.map(NearToken::from_yoctonear),
                receiver_id,
                method_names: map_vec_str(method_names),
            },
            LogicMockAction::AddKeyWithFullAccess { receipt_index, public_key, nonce } => {
                Self::AddKeyWithFullAccess { receipt_index, public_key, nonce }
            }
            LogicMockAction::YieldCreate { data_id, receiver_id } => {
                Self::YieldCreate { data_id, receiver_id }
            }
            LogicMockAction::YieldResume { data, data_id } => Self::YieldResume { data, data_id },
        }
    }
}
