use near_sdk::{env, near, Promise};

#[derive(Debug, PartialEq, Eq)]
#[near(serializers=[borsh, json])]
pub struct Pair(u32, u32);

#[derive(Default)]
#[near(serializers=[borsh, json], contract_state(key = b""))]
pub struct Adder {}

#[near]
impl Adder {
    /// Call functions a, b, and c, d,  asynchronously and handle results with `add_callback_vec`.
    pub fn call_all() -> Promise {
        Self::ext(env::current_account_id())
            .a()
            .and(Self::ext(env::current_account_id()).b())
            .and(Self::ext(env::current_account_id()).c())
            .and(Self::ext(env::current_account_id()).d())
            .then(Self::ext(env::current_account_id()).add_callback_vec())
    }

    /// Adds two pairs point-wise.
    pub fn a(&self) -> Pair {
        Pair(1, 1)
    }

    pub fn b(&self) -> Pair {
        Pair(2, 3)
    }

    pub fn c(&self) -> Pair {
        Pair(5, 8)
    }

    pub fn d(&self) -> Pair {
        Pair(13, 21)
    }

    pub fn add_callback_vec(&self, #[callback_vec] elements: Vec<Pair>) -> Pair {
        let start = Pair(0, 0);
        elements.iter().fold(start, |acc, el| sum_pair(&acc, &el))
    }
}

fn sum_pair(a: &Pair, b: &Pair) -> Pair {
    Pair(a.0 + b.0, a.1 + b.1)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use crate::Pair;

    #[tokio::test]
    async fn test_add_callback_vec() -> anyhow::Result<()> {
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

        let result = res.json::<Pair>()?;
        assert_eq!(result, Pair(21, 33));

        Ok(())
    }
}
