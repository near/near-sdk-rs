use near_sdk::__private::schemars::JsonSchema;
use near_sdk::borsh::{self, BorshDeserialize, BorshSchema, BorshSerialize};
use near_sdk::near_bindgen;
use near_sdk::serde::{Deserialize, Serialize};

#[derive(JsonSchema, Serialize, Deserialize, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct Pair(u32, u32);

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct DoublePair {
    first: Pair,
    second: Pair,
}

#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize)]
pub struct Adder {}

#[near_bindgen]
impl Adder {
    /// Adds two pairs point-wise.
    pub fn add(&self, a: Pair, b: Pair) -> Pair {
        sum_pair(&a, &b)
    }

    #[result_serializer(borsh)]
    pub fn add_borsh(&self, #[serializer(borsh)] a: Pair, #[serializer(borsh)] b: Pair) -> Pair {
        sum_pair(&a, &b)
    }

    pub fn add_callback(
        &self,
        #[callback_unwrap] a: DoublePair,
        #[callback_unwrap] b: DoublePair,
        #[callback_vec] others: Vec<DoublePair>,
    ) -> DoublePair {
        Some(b).iter().chain(others.iter()).fold(a, |acc, el| DoublePair {
            first: sum_pair(&acc.first, &el.first),
            second: sum_pair(&acc.second, &el.second),
        })
    }
}

fn sum_pair(a: &Pair, b: &Pair) -> Pair {
    Pair(a.0 + b.0, a.1 + b.1)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use near_abi::*;
    use tokio::fs;
    use workspaces::prelude::*;

    #[ignore]
    #[tokio::test]
    async fn embedded_abi_test() -> anyhow::Result<()> {
        let wasm = fs::read("res/abi.wasm").await?;
        let worker = workspaces::sandbox().await?;
        let contract = worker.dev_deploy(&wasm).await?;

        let res = contract.view(&worker, "__contract_abi", vec![]).await?;
        let abi_root = serde_json::from_slice::<AbiRoot>(&res.result).unwrap();

        assert_eq!(abi_root.schema_version, "0.1.0");
        assert_eq!(abi_root.metadata.name, Some("abi".to_string()));
        assert_eq!(abi_root.metadata.version, Some("0.1.0".to_string()));
        assert_eq!(
            &abi_root.metadata.authors[..],
            &["Near Inc <hello@nearprotocol.com>".to_string()]
        );
        assert_eq!(abi_root.abi.functions.len(), 3);

        let add_function = &abi_root.abi.functions[0];

        assert_eq!(add_function.name, "add".to_string());
        assert_eq!(add_function.doc, Some(" Adds two pairs point-wise.".to_string()));
        assert!(add_function.is_view);
        assert!(!add_function.is_init);
        assert!(!add_function.is_payable);
        assert!(!add_function.is_private);
        assert_eq!(add_function.params.len(), 2);
        assert_eq!(add_function.params[0].name, "a".to_string());
        assert_eq!(add_function.params[1].name, "b".to_string());

        Ok(())
    }
}
