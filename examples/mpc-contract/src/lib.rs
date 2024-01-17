use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
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
    requests: LookupMap<CryptoHash, SignatureRequest>,
}

impl Default for MpcContract {
    fn default() -> Self {
        Self { requests: LookupMap::new(RecordsKey) }
    }
}

const YIELD_NUM_BLOCKS: u64 = 100;
const DATA_ID_REGISTER: u64 = 0;

#[near_bindgen]
impl MpcContract {
    #[payable]
    pub fn sign(&mut self, payload: String) {
        let account_id = env::signer_account_id();

        let promise =
            env::promise_await_data(account_id.clone(), YIELD_NUM_BLOCKS, DATA_ID_REGISTER);

        log!("Created data-awaiting promise with index {:?}", promise);

        let data_id: CryptoHash =
            env::read_register(DATA_ID_REGISTER).expect("").try_into().expect("");

        log!(
            "request by account_id {} for payload {} is pending with data id {:?}",
            account_id,
            payload,
            data_id
        );

        self.requests.insert(&data_id, &SignatureRequest { account_id, payload });

        env::promise_return(promise);
    }
}
