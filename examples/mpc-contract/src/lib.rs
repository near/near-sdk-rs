use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::collections::TreeMap;
use near_sdk::{
    env, log, near_bindgen, require, serde_json, AccountId, BorshStorageKey, CryptoHash, Gas,
    NearToken, PromiseResult,
};

#[derive(BorshSerialize, BorshStorageKey)]
#[borsh(crate = "near_sdk::borsh")]
struct RecordsKey;

#[derive(BorshDeserialize, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
pub struct SignatureRequest {
    account_id: AccountId,
    payload: String,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
pub struct MpcContract {
    requests: TreeMap<CryptoHash, SignatureRequest>,
}

impl Default for MpcContract {
    fn default() -> Self {
        Self { requests: TreeMap::new(RecordsKey) }
    }
}

// Register used to receive data id from `promise_await_data`.
const DATA_ID_REGISTER: u64 = 0;

// Number of blocks for which `sign` will await a signature response.
const YIELD_NUM_BLOCKS: u64 = 100;

// Prepaid gas for a `sign_on_finish` call
const SIGN_ON_FINISH_CALL_GAS: Gas = Gas::from_tgas(10);

#[near_bindgen]
impl MpcContract {
    /// User-facing API: accepts payload and returns signature
    pub fn sign(&mut self, payload: String) {
        // Create the data-awaiting promise
        let data_promise = env::promise_await_data(YIELD_NUM_BLOCKS, DATA_ID_REGISTER);

        // Record the pending request to be picked up by MPC indexers
        let data_id: CryptoHash =
            env::read_register(DATA_ID_REGISTER).expect("").try_into().expect("");
        self.requests
            .insert(&data_id, &SignatureRequest { account_id: env::signer_account_id(), payload });

        // Add a callback for post-processing
        let callback_promise = env::promise_then(
            data_promise,
            env::current_account_id(),
            "sign_on_finish",
            &serde_json::to_vec(&(data_id,)).unwrap(),
            NearToken::from_near(0),
            SIGN_ON_FINISH_CALL_GAS,
        );

        env::promise_return(callback_promise);
    }

    /// Called by MPC participants to submit a signature
    pub fn sign_respond(&mut self, data_id: String, signature: String) {
        let mut data_id_buf = [0u8; 32];
        hex::decode_to_slice(data_id, &mut data_id_buf).expect("");
        let data_id = data_id_buf;

        require!(self.requests.contains_key(&data_id));

        // check that caller is allowed to respond, signature is valid, etc.
        // ...

        log!("submitting response {} for data id {:?}", &signature, &data_id);
        env::promise_submit_data(&data_id, &signature.into_bytes());
    }

    /// Callback used to clean up internal state after a request is processed
    pub fn sign_on_finish(&mut self, data_id: CryptoHash) -> Option<String> {
        self.requests.remove(&data_id);

        require!(env::promise_results_count() == 1);
        match env::promise_result(0) {
            PromiseResult::Successful(x) => {
                let signature = std::str::from_utf8(&x).unwrap().to_string();
                Some(signature + "_post")
            }
            _ => None,
        }
    }

    /// Helper for local testing; prints all pending requests
    pub fn log_pending_requests(&self) {
        for (data_id, request) in self.requests.iter() {
            log!(
                "{}: account_id={} payload={}",
                hex::encode(data_id),
                request.account_id,
                request.payload
            );
        }
    }
}
