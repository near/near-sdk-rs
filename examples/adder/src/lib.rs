use near_sdk::{env, log, near, Promise};

#[derive(Debug, PartialEq, Eq)]
#[near(serializers=[borsh, json])]
pub struct Pair(u32, u32);

#[derive(Default)]
#[near(serializers=[borsh, json], contract_state)]
pub struct Adder {}

#[near]
impl Adder {
    /// Call functions a, b, and c asynchronously and handle results with `handle_callbacks`.
    pub fn call_all() -> Promise {
        Self::ext(env::current_account_id())
            .a()
            .and(Self::ext(env::current_account_id()).b())
            .and(Self::ext(env::current_account_id()).c())
            .and(Self::ext(env::current_account_id()).d())
            .then(Self::ext(env::current_account_id()).handle_callbacks())
    }

    /// Adds two pairs point-wise.
    pub fn a(&self) -> Pair {
        Pair(0, 0)
    }

    pub fn b(&self) -> Pair {
        Pair(2, 2)
    }

    pub fn c(&self) -> Pair {
        Pair(3, 3)
    }

    pub fn d(&self) -> Pair {
        Pair(4, 4)
    }

    pub fn handle_callbacks(
        &self,
        #[callback_unwrap] a: Pair,
        #[callback_unwrap] b: Pair,
        #[callback_vec] others: Vec<Pair>,
    ) -> CompositeCallbacksResult {
        log!("a : {:#?}", a);
        log!("b : {:#?}", b);
        log!("others : {:#?}", others);
        CompositeCallbacksResult { a, b, others }
    }
}

#[near(serializers=[json])]
pub struct CompositeCallbacksResult {
    a: Pair,
    b: Pair,
    others: Vec<Pair>,
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use near_abi::*;
    use near_sdk::serde_json;

    use crate::{CompositeCallbacksResult, Pair};

    #[tokio::test]
    async fn embedded_abi_test() -> anyhow::Result<()> {
        let wasm = near_workspaces::compile_project("./").await?;
        let worker = near_workspaces::sandbox().await?;
        let contract = worker.dev_deploy(&wasm).await?;

        let res = contract.view("__contract_abi").await?;

        let abi_root = serde_json::from_slice::<AbiRoot>(&zstd::decode_all(&res.result[..])?)?;

        assert_eq!(abi_root.schema_version, "0.4.0");
        assert_eq!(abi_root.metadata.name, Some("callback-vec-demo".to_string()));
        // assert_eq!(abi_root.metadata.version, Some("0.1.0".to_string()));
        // assert_eq!(
        //     &abi_root.metadata.authors[..],
        //     &["Near Inc <hello@nearprotocol.com>"]
        // );
        // assert_eq!(abi_root.body.functions.len(), 3);

        // let add_function = &abi_root.body.functions[0];

        // assert_eq!(add_function.name, "add");
        // assert_eq!(add_function.doc, Some(" Adds two pairs point-wise.".to_string()));
        // assert_eq!(add_function.kind, AbiFunctionKind::View);
        // assert_eq!(add_function.modifiers, &[]);
        // match &add_function.params {
        //     AbiParameters::Json { args } => {
        //         assert_eq!(args.len(), 2);
        //         assert_eq!(args[0].name, "a");
        //         assert_eq!(args[1].name, "b");
        //     }
        //     AbiParameters::Borsh { .. } => {
        //         assert!(false);
        //     }
        // }

        Ok(())
    }

    #[tokio::test]
    async fn test_callback_vec_logic() -> anyhow::Result<()> {
        let wasm = near_workspaces::compile_project("./").await?;
        let worker = near_workspaces::sandbox().await?;
        let contract = worker.dev_deploy(&wasm).await?;

        let res = contract
            .call("call_all")
            .args_json(())
            .gas(near_sdk::Gas::from_tgas(300))
            .transact()
            .await?;
        println!("res: {:#?}", res);

        let composite_result = res.json::<CompositeCallbacksResult>()?;
        assert_eq!(composite_result.a, Pair(0, 0));
        assert_eq!(composite_result.b, Pair(2, 2));
        assert_eq!(composite_result.others, vec![Pair(0, 0), Pair(2, 2), Pair(3, 3), Pair(4, 4),]);

        Ok(())
    }
}
