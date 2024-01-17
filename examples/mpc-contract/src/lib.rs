use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::collections::TreeMap;
use near_sdk::{env, log, near_bindgen, AccountId, BorshStorageKey, CryptoHash};

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
    // Pending requests
    requests: TreeMap<CryptoHash, SignatureRequest>,
}

impl Default for MpcContract {
    fn default() -> Self {
        Self { requests: TreeMap::new(RecordsKey) }
    }
}

const YIELD_NUM_BLOCKS: u64 = 100;
const DATA_ID_REGISTER: u64 = 0;

#[near_bindgen]
impl MpcContract {
    #[payable]
    pub fn sign(&mut self, payload: String) {
        let account_id = env::signer_account_id();

        let promise = env::promise_await_data(&account_id, YIELD_NUM_BLOCKS, DATA_ID_REGISTER);

        let data_id: CryptoHash =
            env::read_register(DATA_ID_REGISTER).expect("").try_into().expect("");

        self.requests.insert(&data_id, &SignatureRequest { account_id, payload });

        env::promise_return(promise);
    }

    #[payable]
    pub fn sign_respond(&mut self, signature: String) {
        // TODO: really the signer should pass the data id,
        // but for now just match this response to an arbitrary pending request
        let data_id = self.requests.min().expect("");

        env::promise_submit_data(&data_id, &signature.into_bytes());
    }
}
