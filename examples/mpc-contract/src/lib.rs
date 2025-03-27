use near_sdk::{
    env, log, near, serde_json, store::IterableMap, AccountId, CryptoHash, Gas, GasWeight,
    NearToken, PromiseError,
};

// Register used to receive data id from `promise_await_data`.
const DATA_ID_REGISTER: u64 = 0;

// Prepaid gas for a `sign_on_finish` call
const SIGN_ON_FINISH_CALL_GAS: Gas = Gas::from_tgas(5);

// Prepaid gas for a `do_something` call
const CHAINED_CALL_GAS: Gas = Gas::from_tgas(5);

#[near(serializers = [borsh, json])]
#[derive(Clone, Debug)]
pub struct SignatureRequest {
    pub data_id: CryptoHash,
    pub account_id: AccountId,
    pub message: String,
}

#[near(contract_state)]
pub struct MpcContract {
    requests: IterableMap<u64, SignatureRequest>,
    next_available_request_index: u64,
}

impl Default for MpcContract {
    fn default() -> Self {
        Self {
            requests: IterableMap::new(b"requests".to_vec()),
            next_available_request_index: 0u64,
        }
    }
}

#[near]
impl MpcContract {
    /// User-facing API: accepts some message and returns a signature
    pub fn sign(&mut self, message: String) {
        let index = self.next_available_request_index;
        self.next_available_request_index += 1;

        let yield_promise = env::promise_yield_create(
            "sign_on_finish",
            &serde_json::to_vec(&(index,)).unwrap(),
            SIGN_ON_FINISH_CALL_GAS,
            GasWeight(0),
            DATA_ID_REGISTER,
        );

        // Store the request in the contract's local state
        let data_id: CryptoHash =
            env::read_register(DATA_ID_REGISTER).expect("").try_into().expect("");
        self.requests.insert(
            index,
            SignatureRequest { data_id, account_id: env::signer_account_id(), message },
        );

        // The yield promise is composable with the usual promise API features. We can choose to
        // chain another function call and it will receive the output of the `sign_on_finish`
        // callback. Note that this chained promise can be a cross-contract call.
        env::promise_then(
            yield_promise,
            env::current_account_id(),
            "do_something",
            &[],
            NearToken::from_near(0),
            CHAINED_CALL_GAS,
        );

        // The return value for this function call will be the value
        // returned by the `sign_on_finish` callback.
        env::promise_return(yield_promise);
    }

    /// Called by MPC participants to submit a signature
    pub fn sign_respond(&mut self, data_id: CryptoHash, signature: String) {
        // check that caller is allowed to respond, signature is valid, etc.
        // ...

        log!("submitting response {} for data id {:?}", &signature, &data_id);
        env::promise_yield_resume(&data_id, &serde_json::to_vec(&signature).unwrap());
    }

    /// Callback receiving the externally submitted data (or a PromiseError)
    pub fn sign_on_finish(
        &mut self,
        request_index: u64,
        #[callback_result] signature: Result<String, PromiseError>,
    ) -> String {
        // Clean up the local state
        self.requests.remove(&request_index);

        match signature {
            Ok(signature) => "signature received: ".to_owned() + &signature,
            Err(_) => "signature request timed out".to_string(),
        }
    }

    pub fn do_something(#[callback_unwrap] signature_result: String) {
        log!("fn do_something invoked with result '{}'", &signature_result);
    }

    /// Helper for local testing; prints all pending requests
    pub fn log_pending_requests(&self) {
        for (_, request) in self.requests.iter() {
            log!(
                "{}: account_id={} payload={}",
                hex::encode(request.data_id),
                request.account_id,
                request.message
            );
        }
    }

    pub fn get_requests(&self) -> Vec<SignatureRequest> {
        self.requests.iter().map(|(_, request)| request).cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use std::{task::Poll, time::Duration};

    use super::*;

    use near_sdk::serde_json;
    use near_workspaces::{network::Sandbox, result::ExecutionFinalResult};

    const SIGNATURE_TEXT: &str = "I'm a cool signature, Kolo?";

    async fn mpc_loop(
        workspace: near_workspaces::Worker<Sandbox>,
        contract: near_workspaces::Contract,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mpc_server = workspace.dev_create_account().await?;
        let mut interval = tokio::time::interval(Duration::from_secs(2));

        for i in 0..10 {
            interval.tick().await;
            println!("MPC server loop iteration {}", i);
            let requests: Vec<SignatureRequest> = contract.view("get_requests").await?.json()?;
            println!("Requests: {:?}", requests);
            if !requests.is_empty() {
                let result = mpc_server.call(contract.id(), "sign_respond")
                    .gas(Gas::from_tgas(200))
                    .args_json(serde_json::json!({ "data_id": requests[0].data_id, "signature": SIGNATURE_TEXT }))
                    .transact()
                    .await?
                    .into_result();
                println!("MPC server result: {:?}", result);
                result?;
                return Ok(());
            }
        }

        Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "No requests found")))
    }

    async fn alice_part(
        workspace: near_workspaces::Worker<Sandbox>,
        contract: near_workspaces::Contract,
    ) -> Result<ExecutionFinalResult, Box<dyn std::error::Error>> {
        let alice = workspace.dev_create_account().await?;
        let mut interval = tokio::time::interval(Duration::from_secs(2));
        let message = "Hello, world!";
        let block_init = workspace.view_block().await?.height();
        let result = alice
            .call(contract.id(), "sign")
            .args_json(serde_json::json!({ "message": message }))
            .gas(Gas::from_tgas(300))
            .transact_async()
            .await?;

        loop {
            interval.tick().await;
            let block = workspace.view_block().await?.height();
            println!("Alice waiting for ~{} blocks", block - block_init);

            let status = result.status().await?;
            if let Poll::Ready(result) = status {
                println!("Alice result: {:?}", result);
                return Ok(result);
            }
        }
    }

    #[tokio::test]
    async fn happy_path() -> Result<(), Box<dyn std::error::Error>> {
        let wasm = near_workspaces::compile_project("./").await?;
        let workspace = near_workspaces::sandbox().await?;

        let contract_account = workspace.dev_create_account().await?;
        let contract = contract_account.deploy(wasm.as_slice()).await?.into_result()?;

        let (alice_result, mpc_result) = tokio::join!(
            // Alice starts the signing process
            alice_part(workspace.clone(), contract.clone()),
            // The yield process is some service that waits for the user request and responds to it
            mpc_loop(workspace.clone(), contract.clone())
        );

        assert!(alice_result.is_ok());
        assert!(mpc_result.is_ok());

        let alice_result = alice_result.unwrap();
        let logs = alice_result.logs();
        assert_eq!(logs.len(), 1);

        // As you can see, for the Alice it looks like a single transaction, but in the reality
        // it waits for the outside entity to respond to continue the execution.
        assert_eq!(
            logs[0],
            format!("fn do_something invoked with result 'signature received: {}'", SIGNATURE_TEXT)
        );
        Ok(())
    }

    #[tokio::test]
    async fn negative_path() -> Result<(), Box<dyn std::error::Error>> {
        let wasm = near_workspaces::compile_project("./").await?;
        let workspace = near_workspaces::sandbox().await?;

        let contract_account = workspace.dev_create_account().await?;
        let contract = contract_account.deploy(wasm.as_slice()).await?.into_result()?;

        // Alice starts the signing process, but the MPC server does not respond
        // for quite some time, so the transaction times out.
        // This is managed by protocol-level parameter `yield_timeout_length_in_blocks`.
        let alice_result = alice_part(workspace, contract).await?;

        let logs = alice_result.logs();
        assert!(alice_result.is_success());
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0], "fn do_something invoked with result 'signature request timed out'");
        Ok(())
    }
}
