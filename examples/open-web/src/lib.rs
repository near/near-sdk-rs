use borsh::{BorshDeserialize, BorshSerialize};
use near_bindgen::collections::{Map, Vector};
use near_bindgen::{env, ext_contract, near_bindgen, Promise, PromiseOrValue};
use serde::{Deserialize, Serialize};

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

type PublicKey = Vec<u8>;
type AppId = String;
type Key = String;
type Value = String;
type Message = String;
type AccountId = String;

const GAS: u64 = 1_000_000_000_000_000;

const APP_METHODS: &[u8] = b"set,remove,pull_message,send_message";

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct UserData {
    /// App specific PublicKey -> App ID prefix.
    pub app_keys: Map<PublicKey, AppId>,

    /// Number of app keys per app_id.
    pub num_app_keys: Map<AppId, u64>,

    /// Received messages. Should be a queue, but whatever.
    pub messages: Map<AppId, Vector<WrappedMessage>>,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub struct WrappedMessage {
    pub sender: AccountId,
    pub message: Message,
    pub time: u64,
}

fn verify_app_id(app_id: &AppId) {
    if app_id.len() < 2 || app_id.len() > 64 {
        env::panic(b"App ID length should be between 2 and 64 characters");
    }
    for c in app_id.bytes() {
        match c {
            b'a'..=b'z' => (),
            b'0'..=b'9' => (),
            b'-' | b'_' | b'.' => (),
            _ => env::panic(
                b"Unsupported character in the app ID. Only allowed to use `-.|` and 0-9 a-z",
            ),
        }
    }
}

fn app_key(app_id: &AppId, key: &Key) -> Vec<u8> {
    let mut res = Vec::with_capacity(app_id.len() + key.len() + 1);
    res.extend_from_slice(app_id.as_bytes());
    res.push(b':');
    res.extend_from_slice(key.as_bytes());
    res
}

// One can provide a name, e.g. `ext` to use for generated methods.
#[ext_contract(remote)]
pub trait RemoteSelf {
    fn post_message(&mut self, app_id: AppId, message: Message);
}

#[near_bindgen(init => new)]
impl UserData {
    pub fn new() -> Self {
        Self {
            app_keys: Map::new(b":app_keys".to_vec()),
            num_app_keys: Map::new(b":num_app_keys".to_vec()),
            messages: Map::new(b":messages".to_vec()),
        }
    }

    pub fn apps(&self) -> Vec<AppId> {
        self.num_app_keys.keys().collect()
    }

    pub fn set(&mut self, key: Key, value: Value) {
        let app_id = self.auth_app_id();
        env::storage_write(&app_key(&app_id, &key), &value.as_bytes());
    }

    pub fn remove(&mut self, key: Key) {
        let app_id = self.auth_app_id();
        env::storage_remove(&app_key(&app_id, &key));
    }

    pub fn get(&self, app_id: AppId, key: Key) -> Option<Value> {
        verify_app_id(&app_id);
        env::storage_read(&app_key(&app_id, &key)).map(|bytes| String::from_utf8(bytes).unwrap())
    }

    pub fn post_message(&mut self, app_id: AppId, message: Message) {
        verify_app_id(&app_id);
        self.verify_app_active(&app_id);
        let mut q = self.messages.get(&app_id).unwrap_or_else(|| {
            let mut vec_id = Vec::with_capacity(app_id.len() + 4);
            vec_id.extend_from_slice(b":m:");
            vec_id.extend_from_slice(app_id.as_bytes());
            vec_id.push(b':');
            Vector::new(vec_id)
        });
        q.push(&WrappedMessage {
            sender: env::predecessor_account_id(),
            message,
            time: env::block_timestamp(),
        });
        self.messages.insert(&app_id, &q);
    }

    pub fn send_message(
        &mut self,
        receiver_id: AccountId,
        app_id: AppId,
        message: Message,
    ) -> PromiseOrValue<()> {
        let _my_app_id = self.auth_app_id();
        if receiver_id == env::current_account_id() {
            self.post_message(app_id, message);
            PromiseOrValue::Value(())
        } else {
            remote::post_message(app_id, message, &receiver_id, 0, GAS).into()
        }
    }

    pub fn pull_message(&mut self) -> Option<WrappedMessage> {
        let app_id = self.auth_app_id();
        if let Some(mut q) = self.messages.get(&app_id) {
            let message = q.pop();
            if q.is_empty() {
                self.messages.remove(&app_id);
            } else {
                self.messages.insert(&app_id, &q);
            }
            message
        } else {
            None
        }
    }

    pub fn add_app_key(&mut self, public_key: PublicKey, app_id: AppId) -> Promise {
        verify_app_id(&app_id);
        if self.app_keys.insert(&public_key, &app_id).is_some() {
            env::panic(
                b"Public key already exists. Can only enable one app for a given public key",
            );
        }
        self.num_app_keys.insert(&app_id, &(self.num_app_keys.get(&app_id).unwrap_or(0) + 1));
        Promise::new(env::current_account_id()).add_access_key(
            public_key,
            0,                         // allowance
            env::current_account_id(), // receiver_id
            APP_METHODS.to_vec(),      // method_names
        )
    }

    pub fn remove_app_key(&mut self, public_key: PublicKey) {
        let app_id = self.app_keys.remove(&public_key);
        if app_id.is_none() {
            env::panic(b"Public key doesn't exists.");
        }
        let app_id = app_id.unwrap();
        let new_num_app_keys = self.num_app_keys.get(&app_id).unwrap() - 1;
        if new_num_app_keys > 0 {
            self.num_app_keys.insert(&app_id, &new_num_app_keys);
        } else {
            self.num_app_keys.remove(&app_id);
        }
        let promise_id = env::promise_batch_create(&env::current_account_id());
        env::promise_batch_action_delete_key(promise_id, &public_key);
        env::promise_return(promise_id);
    }
}

impl UserData {
    fn auth_app_id(&self) -> AppId {
        let app_id = self.app_keys.get(&env::signer_account_pk());
        if let Some(app_id) = app_id {
            app_id
        } else {
            env::panic(b"The signer's public key is not authorized for any app ID");
        }
    }

    fn verify_app_active(&self, app_id: &AppId) {
        if self.num_app_keys.get(app_id).is_none() {
            env::panic(b"The app ID is not currently active");
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;
    use near_bindgen::MockedBlockchain;
    use near_bindgen::{testing_env, VMContext};

    fn alice() -> String {
        "alice.near".to_string()
    }
    fn bob() -> String {
        "bob.near".to_string()
    }
    fn carol() -> String {
        "carol.near".to_string()
    }

    fn get_context(signer_account_pk: Vec<u8>) -> VMContext {
        VMContext {
            current_account_id: alice(),
            signer_account_id: alice(),
            signer_account_pk,
            predecessor_account_id: alice(),
            input: vec![],
            block_index: 0,
            block_timestamp: 0,
            account_balance: 0,
            account_locked_balance: 0,
            storage_usage: 10u64.pow(6),
            attached_deposit: 0,
            prepaid_gas: 10u64.pow(18),
            random_seed: vec![0, 1, 2],
            is_view: false,
            output_data_receivers: vec![],
        }
    }

    #[test]
    fn test_set_get() {
        let pk = vec![0, 1, 2];
        testing_env!(get_context(pk.clone()));
        let mut contract = UserData::new();
        contract.app_keys.insert(&pk, &"app".to_string());
        contract.set("key".to_string(), "value".to_string());
        assert_eq!(contract.get("app".to_string(), "key".to_string()), Some("value".to_string()));
    }

    #[test]
    fn test_set_no_key() {
        testing_env!(get_context(vec![0, 1, 2]));
        let mut contract = UserData::new();
        std::panic::catch_unwind(move || {
            contract.set("key".to_string(), "value".to_string());
        })
        .unwrap_err();
    }

    #[test]
    fn test_set_get_new_key() {
        testing_env!(get_context(vec![0, 1, 2]));
        let mut contract = UserData::new();
        let pk = vec![3, 2, 1];
        contract.add_app_key(pk.clone(), "app".to_string());
        testing_env!(get_context(pk));
        contract.set("key".to_string(), "value".to_string());
        assert_eq!(contract.get("app".to_string(), "key".to_string()), Some("value".to_string()));
    }

    #[test]
    fn test_set_get_new_key_different_app() {
        testing_env!(get_context(vec![0, 1, 2]));
        let mut contract = UserData::new();
        let pk = vec![3, 2, 1];
        contract.add_app_key(pk.clone(), "bla".to_string());
        testing_env!(get_context(pk));
        contract.set("key".to_string(), "value".to_string());
        assert_eq!(contract.get("app".to_string(), "key".to_string()), None);
    }

    #[test]
    fn test_remove_key() {
        let pk = vec![0, 1, 2];
        testing_env!(get_context(pk.clone()));
        let mut contract = UserData::new();
        contract.app_keys.insert(&pk, &"app".to_string());
        contract.num_app_keys.insert(&"app".to_string(), &1);
        contract.remove_app_key(pk);
        std::panic::catch_unwind(move || {
            contract.set("key".to_string(), "value".to_string());
        })
        .unwrap_err();
    }

    #[test]
    fn test_verify_app() {
        testing_env!(get_context(vec![0, 1, 2]));
        let valid_app_ids = vec![
            "hi".to_string(),
            "app".to_string(),
            "a.a".to_string(),
            "blabla".to_string(),
            "_1-2_31231231231231231231231abz23.123123123123123123123".to_string(),
        ];
        for v in valid_app_ids {
            verify_app_id(&v);
        }
        let invalid_app_ids = vec![
            "h".to_string(),
            "".to_string(),
            "a a".to_string(),
            ":app".to_string(),
            "Baa".to_string(),
            "1231231230123123123012312312301231231230123123123012312312301231231230".to_string(),
        ];
        for v in invalid_app_ids {
            std::panic::catch_unwind(move || {
                verify_app_id(&v);
            })
            .unwrap_err();
        }
    }

    #[test]
    fn test_post_pull() {
        let pk = vec![0, 1, 2];
        testing_env!(get_context(pk.clone()));
        let mut contract = UserData::new();
        contract.add_app_key(pk, "app".to_string());
        contract.post_message("app".to_string(), "hello".to_string());
        assert_eq!(contract.pull_message(), Some("hello".to_string()));
        assert_eq!(contract.pull_message(), None);
    }

    #[test]
    fn test_post_pull_different_app() {
        let pk = vec![0, 1, 2];
        let pk2 = vec![3, 2, 1];
        testing_env!(get_context(pk.clone()));
        let mut contract = UserData::new();
        contract.add_app_key(pk, "app".to_string());
        contract.add_app_key(pk2, "bla".to_string());
        contract.post_message("bla".to_string(), "hello".to_string());
        assert_eq!(contract.pull_message(), None);
    }

    #[test]
    fn test_send_message() {
        let pk = vec![0, 1, 2];
        let pk2 = vec![3, 2, 1];
        testing_env!(get_context(pk.clone()));
        let mut contract = UserData::new();
        contract.add_app_key(pk, "app".to_string());
        contract.add_app_key(pk2.clone(), "bla".to_string());
        contract.send_message(alice(), "bla".to_string(), "hello".to_string());
        // Goes to somewhere else
        contract.send_message(bob(), "bla".to_string(), "hello".to_string());
        assert_eq!(contract.pull_message(), None);
        testing_env!(get_context(pk2));
        assert_eq!(contract.pull_message(), Some("hello".to_string()));
        assert_eq!(contract.pull_message(), None);
    }
}
