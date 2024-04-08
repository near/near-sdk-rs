impl IncrementerExt {
    pub fn inc(self) -> ::near_sdk::Promise {
        let __args = ::std::vec![];
        ::near_sdk::Promise::new(self.account_id).function_call_weight(
            ::std::string::String::from("inc"),
            __args,
            self.deposit,
            self.static_gas,
            self.gas_weight,
        )
    }
    pub fn get(self) -> ::near_sdk::Promise {
        let __args = ::std::vec![];
        ::near_sdk::Promise::new(self.account_id).function_call_weight(
            ::std::string::String::from("get"),
            __args,
            self.deposit,
            self.static_gas,
            self.gas_weight,
        )
    }
    pub fn top(self) -> ::near_sdk::Promise {
        let __args = ::std::vec![];
        ::near_sdk::Promise::new(self.account_id).function_call_weight(
            ::std::string::String::from("top"),
            __args,
            self.deposit,
            self.static_gas,
            self.gas_weight,
        )
    }
}
impl Incrementer {
    pub fn inc(&mut self) -> Result<String, String> {
        self.value += 1;
        Ok("ok".to_string())
    }
    pub fn get(&mut self) -> Result<String, String> {
        self.value += 1;
        Ok("hey".to_string())
    }
    pub fn top(&mut self) {
        self.value += 1;
    }
}
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn inc() {
    ::near_sdk::env::setup_panic_hook();
    if ::near_sdk::env::attached_deposit().as_yoctonear() != 0 {
        ::near_sdk::env::panic_str("Method inc doesn't accept deposit");
    }
    let mut contract: Incrementer = ::near_sdk::env::state_read().unwrap_or_default();
    let result = contract.inc();
    match result {
        ::std::result::Result::Ok(result) => {
            let result = ::near_sdk::serde_json::to_vec(&result)
                .expect("Failed to serialize the return value using JSON.");
            ::near_sdk::env::value_return(&result);
            ::near_sdk::env::state_write(&contract);
        }
        ::std::result::Result::Err(err) => ::near_sdk::FunctionError::panic(&err),
    }
}
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn get() {
    ::near_sdk::env::setup_panic_hook();
    if ::near_sdk::env::attached_deposit().as_yoctonear() != 0 {
        ::near_sdk::env::panic_str("Method get doesn't accept deposit");
    }
    let mut contract: Incrementer = ::near_sdk::env::state_read().unwrap_or_default();
    let result = contract.get();
    match result {
        ::std::result::Result::Ok(result) => {
            let result = ::near_sdk::serde_json::json!({
                "status": "Success",
                "result": result,
            })
            let result = ::near_sdk::serde_json::to_vec(&result)
                .expect("Failed to serialize the return value using JSON.");
            ::near_sdk::env::value_return(&result);
            ::near_sdk::env::state_write(&contract);
        }
        ::std::result::Result::Err(err) => ::near_sdk::FunctionError::panic(&err),
    }
}
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn top() {
    ::near_sdk::env::setup_panic_hook();
    if ::near_sdk::env::attached_deposit().as_yoctonear() != 0 {
        ::near_sdk::env::panic_str("Method top doesn't accept deposit");
    }
    let mut contract: Incrementer = ::near_sdk::env::state_read().unwrap_or_default();
    contract.top();
    ::near_sdk::env::state_write(&contract);
}
