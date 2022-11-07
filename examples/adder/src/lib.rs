use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{near_bindgen, NearSchema};

#[derive(NearSchema, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
#[abi(json, borsh)]
pub struct Pair(u32, u32);

#[derive(NearSchema, Serialize, Deserialize)]
#[abi(json, borsh)]
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

    #[ignore]
    #[tokio::test]
    async fn embedded_abi_test() -> anyhow::Result<()> {
        let wasm = fs::read("res/adder.wasm").await?;
        let worker = workspaces::sandbox().await?;
        let contract = worker.dev_deploy(&wasm).await?;

        let res = contract.view("__contract_abi", vec![]).await?;

        let abi_root =
            serde_json::from_slice::<AbiRoot>(&zstd::decode_all(&res.result[..])?).unwrap();

        assert_eq!(abi_root.schema_version, "0.3.0");
        assert_eq!(abi_root.metadata.name, Some("adder".to_string()));
        assert_eq!(abi_root.metadata.version, Some("0.1.0".to_string()));
        assert_eq!(
            &abi_root.metadata.authors[..],
            &["Near Inc <hello@nearprotocol.com>".to_string()]
        );
        assert_eq!(abi_root.body.functions.len(), 3);

        let add_function = &abi_root.body.functions[0];

        assert_eq!(add_function.name, "add".to_string());
        assert_eq!(add_function.doc, Some(" Adds two pairs point-wise.".to_string()));
        assert_eq!(add_function.kind, AbiFunctionKind::View);
        assert_eq!(add_function.modifiers, vec![]);
        match &add_function.params {
            AbiParameters::Json { args } => {
                assert_eq!(args.len(), 2);
                assert_eq!(args[0].name, "a".to_string());
                assert_eq!(args[1].name, "b".to_string());
            }
            AbiParameters::Borsh { .. } => {
                assert!(false);
            }
        }

        Ok(())
    }
}
