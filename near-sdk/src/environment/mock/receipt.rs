use near_primitives_core::types::GasWeight;
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
        method_names: Vec<Vec<u8>>,
    },
    AddKeyWithFullAccess {
        receipt_index: ReceiptIndex,
        public_key: near_crypto::PublicKey,
        nonce: u64,
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
            Self::AddKeyWithFullAccess { receipt_index, .. } => Some(*receipt_index),
        }
    }
}
