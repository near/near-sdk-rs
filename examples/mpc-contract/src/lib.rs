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
const SIGN_ON_FINISH_CALL_GAS: Gas = Gas::from_tgas(5);

// Prepaid gas for a `sign_on_finish` call
const REMOVE_REQUEST_CALL_GAS: Gas = Gas::from_tgas(5);

#[near_bindgen]
impl MpcContract {
    /// User-facing API: accepts payload and returns signature
    pub fn sign(&mut self, payload: String) {
        let promise = env::promise_yield_create(
            "sign_on_finish",
            &[],
            SIGN_ON_FINISH_CALL_GAS,
            YIELD_NUM_BLOCKS,
            DATA_ID_REGISTER,
        );

        // Record the pending request
        let data_id: CryptoHash =
            env::read_register(DATA_ID_REGISTER).expect("").try_into().expect("");
        self.requests
            .insert(&data_id, &SignatureRequest { account_id: env::signer_account_id(), payload });

        // Schedule clean-up of self.requests after the yield execution completes
        env::promise_then(
            promise,
            env::current_account_id(),
            "remove_request",
            &serde_json::to_vec(&(data_id,)).unwrap(),
            NearToken::from_near(0),
            REMOVE_REQUEST_CALL_GAS,
        );

        env::promise_return(promise);
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
        env::promise_yield_resume(&data_id, &signature.into_bytes());
    }

    /// Callback receiving the externally submitted data
    pub fn sign_on_finish(&mut self) -> Option<String> {
        require!(env::promise_results_count() == 1);
        match env::promise_result(0) {
            PromiseResult::Successful(x) => {
                let signature = std::str::from_utf8(&x).unwrap().to_string();
                Some(signature + "_post")
            }
            _ => None,
        }
    }

    /// Callback used to clean up the local state of the contract
    pub fn remove_request(&mut self, data_id: CryptoHash) {
        self.requests.remove(&data_id);
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
