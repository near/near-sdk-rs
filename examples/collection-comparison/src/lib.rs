use near_sdk::{ near_bindgen, env };
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
// Documentation: https://docs.rs/near-sdk/2.0.0/near_sdk/collections/index.html
use near_sdk::collections::{ LookupMap, UnorderedMap, TreeMap };

#[global_allocator]
static ALLOC: near_sdk::wee_alloc::WeeAlloc = near_sdk::wee_alloc::WeeAlloc::INIT;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Collections {
    pub tree_map: TreeMap<String, String>,
    pub unordered_map: UnorderedMap<String, String>,
    pub lookup_map: LookupMap<String, String>
}

impl Default for Collections {
    fn default() -> Self {
        env::panic(b"The contract is not initialized.")
    }
}

#[near_bindgen]
impl Collections {
    /// Init attribute used for instantiation.
    #[init]
    pub fn new() -> Self {
        // Useful snippet to copy/paste, making sure state isn't already initialized
        assert!(env::state_read::<Self>().is_none(), "Already initialized");
        // Note this is an implicit "return" here
        Self {
            tree_map: TreeMap::new(b"t".to_vec()),
            unordered_map: UnorderedMap::new(b"u".to_vec()),
            lookup_map: LookupMap::new(b"l".to_vec())
        }
    }

    // This functions changes state, so 1st param uses `&mut self`
    /// Add data to TreeMap
    pub fn add_tree_map(&mut self, key: String, value: String) {
        self.tree_map.insert(&key, &value);
    }

    /// Add data to UnorderedMap
    pub fn add_unordered_map(&mut self, key: String, value: String) {
        self.unordered_map.insert(&key, &value);
    }

    /// Add data to LookupMap
    pub fn add_lookup_map(&mut self, key: String, value: String) {
        self.lookup_map.insert(&key, &value);
    }

    // This functions simple returns state, so 1st param uses `&self`
    /// Return entry from TreeMap
    pub fn get_tree_map(&self, key: String) -> String {
        match self.tree_map.get(&key) {
            Some(value) => {
                let log_message = format!("Value from TreeMap is {:?}", value.clone());
                env::log(log_message.as_bytes());
                // Since we found it, return it (note implicit return)
                value
            },
            // did not find the entry
            // note: curly brackets after arrow are optional in simple cases, like other languages
            None => "none found".to_string()
        }
    }

    /// Return entry from UnorderedMap
    pub fn get_unordered_map(&self, key: String) -> String {
        match self.unordered_map.get(&key) {
            Some(value) => {
                let log_message = format!("Value from UnorderedMap is {:?}", value.clone());
                env::log(log_message.as_bytes());
                value
            },
            None => "none found".to_string()
        }
    }

    /// Return entry from LookupMap
    pub fn get_lookup_map(&self, key: String) -> String {
        match self.lookup_map.get(&key) {
            Some(value) => {
                let log_message = format!("Value from LookupMap is {:?}", value.clone());
                env::log(log_message.as_bytes());
                value
            },
            None => "none found".to_string()
        }
    }

    /// Reset the data structure(s)
    /// if `tree_map` is `true` this function will clear the values
    /// Same for `unordered_map` but `lookup_map` does not have this
    pub fn reset(&mut self, tree_map: bool, unordered_map: bool) {
        assert_eq!(env::current_account_id(), env::predecessor_account_id(), "This method must be called by the (implied) contract owner.");
        if tree_map {
            self.tree_map.clear();
        }
        if unordered_map {
            self.unordered_map.clear();
        }
    }
}