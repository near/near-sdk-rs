//! Contract whose wasm32 method wrappers must not be captured by user consts in scope.
//!
//! The `extern "C"` wrappers `#[near]` generates only exist for `target_arch = "wasm32"`, so
//! `tests/macro_hygiene.rs` checks this crate for that target. Each const below is named like a
//! binding the wrappers used to generate; a plain `let`, match arm or closure parameter with that
//! name resolves to the const instead and fails to compile (E0005/E0308).

#[allow(non_upper_case_globals, dead_code)]
mod consts {
    pub const contract: u64 = 0;
    pub const result: u64 = 0;
    pub const input: u64 = 0;
    pub const deserialized: u64 = 0;
    pub const e: u64 = 0;
    pub const err: u64 = 0;
    pub const v: u64 = 0;
    pub const data: u64 = 0;
    pub const i: u64 = 0;
}

#[allow(unused_imports)]
use consts::*;
use near_sdk::{PromiseError, near};

#[near(contract_state)]
#[derive(Default)]
pub struct Contract {
    n: u8,
}

#[near]
impl Contract {
    #[init]
    pub fn new(n: u8) -> Self {
        Self { n }
    }

    #[init]
    #[handle_result]
    pub fn try_new(n: u8) -> Result<Self, &'static str> {
        if n == 0 { Err("zero") } else { Ok(Self { n }) }
    }

    pub fn get(&self) -> u8 {
        self.n
    }

    #[result_serializer(borsh)]
    pub fn get_borsh(&self) -> u8 {
        self.n
    }

    pub fn set(&mut self, n: u8) {
        self.n = n;
    }

    pub fn set_borsh(&mut self, #[serializer(borsh)] n: u8) {
        self.n = n;
    }

    #[handle_result]
    pub fn checked(&self) -> Result<u8, &'static str> {
        if self.n == 0 { Err("zero") } else { Ok(self.n) }
    }

    #[handle_result]
    pub fn checked_set(&mut self, n: u8) -> Result<u8, &'static str> {
        if n == 0 {
            return Err("zero");
        }
        self.n = n;
        Ok(n)
    }

    pub fn add(a: u8, b: u8) -> u8 {
        a.wrapping_add(b)
    }

    #[payable]
    pub fn deposit(&mut self) {
        self.n = self.n.wrapping_add(1);
    }

    #[private]
    pub fn on_unwrap(&mut self, #[callback_unwrap] n: u8) {
        self.n = n;
    }

    #[private]
    pub fn on_unwrap_borsh(
        &mut self,
        #[callback_unwrap]
        #[serializer(borsh)]
        n: u8,
    ) {
        self.n = n;
    }

    #[private]
    pub fn on_result(
        &mut self,
        #[callback_result] n: Result<u8, PromiseError>,
        #[callback_result] unit: Result<(), PromiseError>,
    ) -> bool {
        if let Ok(n) = n {
            self.n = n;
        }
        unit.is_ok()
    }

    #[private]
    pub fn on_vec(&mut self, #[callback_vec] ns: Vec<u8>) {
        self.n = ns.into_iter().fold(0, u8::wrapping_add);
    }
}
