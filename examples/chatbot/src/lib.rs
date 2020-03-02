use borsh::{BorshDeserialize, BorshSerialize};
use near_bindgen::collections::{Set, Vector};
use near_bindgen::{env, ext_contract, near_bindgen};
use serde::{Deserialize, Serialize};

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

type AppId = String;
type Key = String;
type Value = String;
type Message = String;
type AccountId = String;

const GAS: u64 = 100_000_000_000_000;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct ChatBot {
    previous_messages: Vector<Message>,
    senders: Set<AccountId>,
    random_seed: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct UnencryptedMessage {
    #[serde(rename = "type")]
    ftype: String,
    subject: String,
    content: String,
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

#[ext_contract(remote)]
pub trait RemoteSelf {
    fn post_message(&mut self, app_id: String, message: String);
}

impl Default for ChatBot {
    fn default() -> Self {
        env::panic(b"Not initialized yet.");
    }
}

fn assert_self() {
    assert_eq!(env::current_account_id(), env::predecessor_account_id(), "Self calls only");
}

#[near_bindgen]
impl ChatBot {
    #[init]
    pub fn new() -> Self {
        assert!(env::state_read::<ChatBot>().is_none(), "The contract is already initialized");
        Self {
            previous_messages: Vector::new(b":pm:".to_vec()),
            senders: Set::new(b":s:".to_vec()),
            random_seed: env::random_seed(),
        }
    }

    pub fn master_set(&mut self, app_id: AppId, key: Key, value: Value) {
        assert_self();
        env::storage_write(&app_key(&app_id, &key), &value.as_bytes());
    }

    pub fn master_remove(&mut self, app_id: AppId, key: Key) {
        assert_self();
        env::storage_remove(&app_key(&app_id, &key));
    }

    pub fn get(&self, app_id: AppId, key: Key) -> Option<Value> {
        verify_app_id(&app_id);
        env::storage_read(&app_key(&app_id, &key)).map(|bytes| String::from_utf8(bytes).unwrap())
    }

    pub fn post_message(&mut self, app_id: AppId, message: Message) {
        verify_app_id(&app_id);
        assert_eq!(app_id.as_bytes(), b"mail", "I only support mail messages");

        let sender_id = env::predecessor_account_id();
        self.add_sender(&sender_id);

        let message: UnencryptedMessage = match serde_json::from_str(&message) {
            Ok(res) => res,
            Err(e) => {
                self.mail_message(
                    &sender_id,
                    "An error has occurred :(".to_string(),
                    format!("Sorry, something is off. Can't parse it:\n{}", e),
                );
                return;
            }
        };
        let message = if !message.content.is_empty() {
            message.content.lines().next().unwrap().to_string()
        } else {
            message.subject
        };
        self.add_message(&message);
        self.mail_random_message(&sender_id, Some(message));
        let random_sender_id = self.random_sender_id();
        if sender_id != random_sender_id {
            self.mail_random_message(&random_sender_id, None);
        }
    }

    pub fn show_me_senders(&self) -> Vec<AccountId> {
        self.senders.to_vec()
    }

    pub fn num_messages(&self) -> u64 {
        self.previous_messages.len()
    }
}

impl ChatBot {
    fn add_sender(&mut self, sender_id: &AccountId) {
        self.senders.insert(&sender_id);
    }

    fn add_message(&mut self, message: &Message) {
        self.previous_messages.push(&message);
    }

    fn random_u64(&mut self) -> u64 {
        self.random_seed = env::sha256(&self.random_seed);
        let mut v = [0u8; 8];
        v.copy_from_slice(&self.random_seed[..8]);
        u64::from_le_bytes(v)
    }

    fn random_sender_id(&mut self) -> AccountId {
        let rnd = self.random_u64();
        let v = self.senders.as_vector();
        v.get(rnd % v.len()).unwrap()
    }

    fn random_message(&mut self) -> Message {
        let rnd = self.random_u64();
        let v = &self.previous_messages;
        v.get(rnd % v.len()).unwrap()
    }

    fn mail_random_message(&mut self, receiver_id: &AccountId, subject: Option<Message>) {
        let message = self.random_message();
        if let Some(subject) = subject {
            self.mail_message(&receiver_id, format!("Re: {}", subject), message);
        } else {
            let content = self.random_message();
            self.mail_message(&receiver_id, message, content);
        }
    }

    fn mail_message(&self, receiver_id: &AccountId, subject: String, content: Message) {
        let message = serde_json::to_string(&UnencryptedMessage {
            ftype: "mail".to_string(),
            subject,
            content,
        })
        .unwrap();
        remote::post_message("mail".to_string(), message, &receiver_id, 0, GAS);
    }
}
