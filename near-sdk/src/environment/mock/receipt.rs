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
    #[cfg(feature = "global-contracts")]
    DeployGlobalContract {
        receipt_index: ReceiptIndex,
        code: Vec<u8>,
    },
    #[cfg(feature = "global-contracts")]
    UseGlobalContract {
        receipt_index: ReceiptIndex,
        // Store as String to avoid trait bound issues with GlobalContractIdentifier
        contract_id: String,
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
            #[cfg(feature = "global-contracts")]
            MockAction::DeployGlobalContract { receipt_index, .. } => Some(*receipt_index),
            #[cfg(feature = "global-contracts")]
            MockAction::UseGlobalContract { receipt_index, .. } => Some(*receipt_index),
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
            #[cfg(feature = "global-contracts")]
            LogicMockAction::DeployGlobalContract { receipt_index, code, .. } => {
                Self::DeployGlobalContract { receipt_index, code }
            }
            #[cfg(feature = "global-contracts")]
            LogicMockAction::UseGlobalContract { receipt_index, contract_id, .. } => {
                Self::UseGlobalContract { receipt_index, contract_id: format!("{:?}", contract_id) }
            }
            #[cfg(not(feature = "global-contracts"))]
            LogicMockAction::DeployGlobalContract { .. } => {
                panic!("Global contract functionality requires the 'global-contracts' feature flag")
            }
            #[cfg(not(feature = "global-contracts"))]
            LogicMockAction::UseGlobalContract { .. } => {
                panic!("Global contract functionality requires the 'global-contracts' feature flag")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_vm_runner::logic::mocks::mock_external::MockAction as LogicMockAction;

    #[cfg(feature = "global-contracts")]
    #[test]
    fn test_global_contract_mock_actions() {
        let deploy_action =
            MockAction::DeployGlobalContract { receipt_index: 0, code: vec![1, 2, 3] };
        assert_eq!(deploy_action.receipt_index(), Some(0));

        let use_action = MockAction::UseGlobalContract {
            receipt_index: 1,
            contract_id: "test_contract".to_string(),
        };
        assert_eq!(use_action.receipt_index(), Some(1));
    }

    #[cfg(feature = "global-contracts")]
    #[test]
    fn test_logic_mock_action_conversion() {
        let logic_deploy = LogicMockAction::DeployGlobalContract {
            receipt_index: 0,
            code: vec![1, 2, 3],
            mode: near_vm_runner::logic::types::GlobalContractDeployMode::CodeHash,
        };
        let mock_deploy = MockAction::from(logic_deploy);
        assert!(matches!(mock_deploy, MockAction::DeployGlobalContract { .. }));

        let logic_use = LogicMockAction::UseGlobalContract {
            receipt_index: 1,
            contract_id: near_vm_runner::logic::types::GlobalContractIdentifier::AccountId(
                "test_contract".parse().unwrap(),
            ),
        };
        let mock_use = MockAction::from(logic_use);
        assert!(matches!(mock_use, MockAction::UseGlobalContract { .. }));
    }

    #[cfg(not(feature = "global-contracts"))]
    #[test]
    #[should_panic(
        expected = "Global contract functionality requires the 'global-contracts' feature flag"
    )]
    fn test_deploy_global_contract_panic_without_feature() {
        let logic_action = LogicMockAction::DeployGlobalContract {
            receipt_index: 0,
            code: vec![1, 2, 3],
            mode: near_vm_runner::logic::types::GlobalContractDeployMode::CodeHash,
        };
        let _ = MockAction::from(logic_action);
    }

    #[cfg(not(feature = "global-contracts"))]
    #[test]
    #[should_panic(
        expected = "Global contract functionality requires the 'global-contracts' feature flag"
    )]
    fn test_use_global_contract_panic_without_feature() {
        let logic_action = LogicMockAction::UseGlobalContract {
            receipt_index: 1,
            contract_id: near_vm_runner::logic::types::GlobalContractIdentifier::AccountId(
                "test_contract".parse().unwrap(),
            ),
        };
        let _ = MockAction::from(logic_action);
    }
}
