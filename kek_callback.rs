impl CallbackExt {
    pub fn contract_source_metadata(self) -> ::near_sdk::Promise {
        let __args = ::std::vec![];
        ::near_sdk::Promise::new(self.account_id).function_call_weight(
            ::std::string::String::from("contract_source_metadata"),
            __args,
            self.deposit,
            self.static_gas,
            self.gas_weight,
        )
    }
}
impl Callback {
    pub fn contract_source_metadata() {
        near_sdk::env::value_return(CONTRACT_SOURCE_METADATA.as_bytes())
    }
}
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn contract_source_metadata() {
    ::near_sdk::env::setup_panic_hook();
    Callback::contract_source_metadata();
}
#[derive(
    :: near_sdk :: borsh :: BorshSerialize,
    :: near_sdk :: borsh ::
BorshDeserialize,
)]
#[borsh(crate = ":: near_sdk :: borsh")]
pub struct Callback;
#[must_use]
pub struct CallbackExt {
    pub(crate) account_id: ::near_sdk::AccountId,
    pub(crate) deposit: ::near_sdk::NearToken,
    pub(crate) static_gas: ::near_sdk::Gas,
    pub(crate) gas_weight: ::near_sdk::GasWeight,
}
impl CallbackExt {
    pub fn with_attached_deposit(mut self, amount: ::near_sdk::NearToken) -> Self {
        self.deposit = amount;
        self
    }
    pub fn with_static_gas(mut self, static_gas: ::near_sdk::Gas) -> Self {
        self.static_gas = static_gas;
        self
    }
    pub fn with_unused_gas_weight(mut self, gas_weight: u64) -> Self {
        self.gas_weight = ::near_sdk::GasWeight(gas_weight);
        self
    }
}
impl Callback {
    #[doc = r" API for calling this contract's functions in a subsequent execution."]
    pub fn ext(account_id: ::near_sdk::AccountId) -> CallbackExt {
        CallbackExt {
            account_id,
            deposit: ::near_sdk::NearToken::from_near(0),
            static_gas: ::near_sdk::Gas::from_gas(0),
            gas_weight: ::near_sdk::GasWeight::default(),
        }
    }
}
pub const CONTRACT_SOURCE_METADATA : & 'static str =
"{\"version\":\"0.1.0\",\"link\":null,\"standards\":[{\"standard\":\"nep330\",\"version\":\"1.1.0\"}]}"
;
impl CallbackExt {
    pub fn contract_source_metadata(self) -> ::near_sdk::Promise {
        let __args = ::std::vec![];
        ::near_sdk::Promise::new(self.account_id).function_call_weight(
            ::std::string::String::from("contract_source_metadata"),
            __args,
            self.deposit,
            self.static_gas,
            self.gas_weight,
        )
    }
}
impl Callback {
    pub fn contract_source_metadata() {
        near_sdk::env::value_return(CONTRACT_SOURCE_METADATA.as_bytes())
    }
}
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn contract_source_metadata() {
    ::near_sdk::env::setup_panic_hook();
    Callback::contract_source_metadata();
}
impl CallbackExt {
    pub fn call_all(self, fail_b: bool, c_value: u8, d_value: u8) -> ::near_sdk::Promise {
        let __args = {
            #[derive(:: near_sdk :: serde :: Serialize)]
            #[serde(crate = "::near_sdk::serde")]
            struct Input<'nearinput> {
                fail_b: &'nearinput bool,
                c_value: &'nearinput u8,
                d_value: &'nearinput u8,
            }
            let __args = Input { fail_b: &fail_b, c_value: &c_value, d_value: &d_value };
            ::near_sdk::serde_json::to_vec(&__args)
                .expect("Failed to serialize the cross contract args using JSON.")
        };
        ::near_sdk::Promise::new(self.account_id).function_call_weight(
            ::std::string::String::from("call_all"),
            __args,
            self.deposit,
            self.static_gas,
            self.gas_weight,
        )
    }
    pub fn a(self) -> ::near_sdk::Promise {
        let __args = ::std::vec![];
        ::near_sdk::Promise::new(self.account_id).function_call_weight(
            ::std::string::String::from("a"),
            __args,
            self.deposit,
            self.static_gas,
            self.gas_weight,
        )
    }
    pub fn b(self, fail: bool) -> ::near_sdk::Promise {
        let __args = {
            #[derive(:: near_sdk :: serde :: Serialize)]
            #[serde(crate = "::near_sdk::serde")]
            struct Input<'nearinput> {
                fail: &'nearinput bool,
            }
            let __args = Input { fail: &fail };
            ::near_sdk::serde_json::to_vec(&__args)
                .expect("Failed to serialize the cross contract args using JSON.")
        };
        ::near_sdk::Promise::new(self.account_id).function_call_weight(
            ::std::string::String::from("b"),
            __args,
            self.deposit,
            self.static_gas,
            self.gas_weight,
        )
    }
    pub fn c(self, value: u8) -> ::near_sdk::Promise {
        let __args = {
            #[derive(:: near_sdk :: serde :: Serialize)]
            #[serde(crate = "::near_sdk::serde")]
            struct Input<'nearinput> {
                value: &'nearinput u8,
            }
            let __args = Input { value: &value };
            ::near_sdk::serde_json::to_vec(&__args)
                .expect("Failed to serialize the cross contract args using JSON.")
        };
        ::near_sdk::Promise::new(self.account_id).function_call_weight(
            ::std::string::String::from("c"),
            __args,
            self.deposit,
            self.static_gas,
            self.gas_weight,
        )
    }
    pub fn d(self, value: u8) -> ::near_sdk::Promise {
        let __args = {
            #[derive(:: near_sdk :: serde :: Serialize)]
            #[serde(crate = "::near_sdk::serde")]
            struct Input<'nearinput> {
                value: &'nearinput u8,
            }
            let __args = Input { value: &value };
            ::near_sdk::serde_json::to_vec(&__args)
                .expect("Failed to serialize the cross contract args using JSON.")
        };
        ::near_sdk::Promise::new(self.account_id).function_call_weight(
            ::std::string::String::from("d"),
            __args,
            self.deposit,
            self.static_gas,
            self.gas_weight,
        )
    }
    pub fn handle_callbacks(self) -> ::near_sdk::Promise {
        let __args = ::std::vec![];
        ::near_sdk::Promise::new(self.account_id).function_call_weight(
            ::std::string::String::from("handle_callbacks"),
            __args,
            self.deposit,
            self.static_gas,
            self.gas_weight,
        )
    }
}
impl Callback {
    #[doc = " Call functions a, b, and c asynchronously and handle results with `handle_callbacks`."]
    pub fn call_all(fail_b: bool, c_value: u8, d_value: u8) -> Promise {
        Self::ext(env::current_account_id())
            .a()
            .and(Self::ext(env::current_account_id()).b(fail_b))
            .and(Self::ext(env::current_account_id()).c(c_value))
            .and(Self::ext(env::current_account_id()).d(d_value))
            .then(Self::ext(env::current_account_id()).handle_callbacks())
    }
    #[doc = " Calls function c with a value that will always succeed"]
    pub fn a() -> Promise {
        Self::ext(env::current_account_id()).c(A_VALUE)
    }
    #[doc = " Returns a static string if fail is false, return"]
    pub fn b(fail: bool) -> &'static str {
        if fail {
            env::panic_str("failed within function b");
        }
        "Some string"
    }
    #[doc = " Panics if value is 0, returns the value passed in otherwise."]
    pub fn c(value: u8) -> u8 {
        require!(value > 0, "Value must be positive");
        value
    }
    #[doc = " Panics if value is 0."]
    pub fn d(value: u8) {
        require!(value > 0, "Value must be positive");
    }
    #[doc = " Receives the callbacks from the other promises called."]
    pub fn handle_callbacks(
        a: u8,
        b: Result<String, PromiseError>,
        c: Result<u8, PromiseError>,
        d: Result<(), PromiseError>,
    ) -> (bool, bool, bool) {
        require!(a == A_VALUE, "Promise returned incorrect value");
        if let Ok(s) = b.as_ref() {
            require!(s == "Some string");
        }
        (b.is_err(), c.is_err(), d.is_err())
    }
}
#[doc = " Call functions a, b, and c asynchronously and handle results with `handle_callbacks`."]
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn call_all() {
    ::near_sdk::env::setup_panic_hook();
    #[derive(:: near_sdk :: serde :: Deserialize)]
    #[serde(crate = "::near_sdk::serde")]
    struct Input {
        fail_b: bool,
        c_value: u8,
        d_value: u8,
    }
    let Input { fail_b, c_value, d_value }: Input = ::near_sdk::serde_json::from_slice(
        &::near_sdk::env::input().expect("Expected input since method has arguments."),
    )
    .expect("Failed to deserialize input from JSON.");
    let result = Callback::call_all(fail_b, c_value, d_value);
    let result = ::near_sdk::serde_json::to_vec(&result)
        .expect("Failed to serialize the return value using JSON.");
    ::near_sdk::env::value_return(&result);
}
#[doc = " Calls function c with a value that will always succeed"]
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn a() {
    ::near_sdk::env::setup_panic_hook();
    let result = Callback::a();
    let result = ::near_sdk::serde_json::to_vec(&result)
        .expect("Failed to serialize the return value using JSON.");
    ::near_sdk::env::value_return(&result);
}
#[doc = " Returns a static string if fail is false, return"]
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn b() {
    ::near_sdk::env::setup_panic_hook();
    if ::near_sdk::env::current_account_id() != ::near_sdk::env::predecessor_account_id() {
        ::near_sdk::env::panic_str("Method b is private");
    }
    #[derive(:: near_sdk :: serde :: Deserialize)]
    #[serde(crate = "::near_sdk::serde")]
    struct Input {
        fail: bool,
    }
    let Input { fail }: Input = ::near_sdk::serde_json::from_slice(
        &::near_sdk::env::input().expect("Expected input since method has arguments."),
    )
    .expect("Failed to deserialize input from JSON.");
    let result = Callback::b(fail);
    let result = ::near_sdk::serde_json::to_vec(&result)
        .expect("Failed to serialize the return value using JSON.");
    ::near_sdk::env::value_return(&result);
}
#[doc = " Panics if value is 0, returns the value passed in otherwise."]
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn c() {
    ::near_sdk::env::setup_panic_hook();
    if ::near_sdk::env::current_account_id() != ::near_sdk::env::predecessor_account_id() {
        ::near_sdk::env::panic_str("Method c is private");
    }
    #[derive(:: near_sdk :: serde :: Deserialize)]
    #[serde(crate = "::near_sdk::serde")]
    struct Input {
        value: u8,
    }
    let Input { value }: Input = ::near_sdk::serde_json::from_slice(
        &::near_sdk::env::input().expect("Expected input since method has arguments."),
    )
    .expect("Failed to deserialize input from JSON.");
    let result = Callback::c(value);
    let result = ::near_sdk::serde_json::to_vec(&result)
        .expect("Failed to serialize the return value using JSON.");
    ::near_sdk::env::value_return(&result);
}
#[doc = " Panics if value is 0."]
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn d() {
    ::near_sdk::env::setup_panic_hook();
    if ::near_sdk::env::current_account_id() != ::near_sdk::env::predecessor_account_id() {
        ::near_sdk::env::panic_str("Method d is private");
    }
    #[derive(:: near_sdk :: serde :: Deserialize)]
    #[serde(crate = "::near_sdk::serde")]
    struct Input {
        value: u8,
    }
    let Input { value }: Input = ::near_sdk::serde_json::from_slice(
        &::near_sdk::env::input().expect("Expected input since method has arguments."),
    )
    .expect("Failed to deserialize input from JSON.");
    Callback::d(value);
}
#[doc = " Receives the callbacks from the other promises called."]
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn handle_callbacks() {
    ::near_sdk::env::setup_panic_hook();
    if ::near_sdk::env::current_account_id() != ::near_sdk::env::predecessor_account_id() {
        ::near_sdk::env::panic_str("Method handle_callbacks is private");
    }
    let data: ::std::vec::Vec<u8> = match ::near_sdk::env::promise_result(0u64) {
        ::near_sdk::PromiseResult::Successful(x) => x,
        _ => ::near_sdk::env::panic_str("Callback computation 0 was not successful"),
    };
    let a: u8 = ::near_sdk::serde_json::from_slice(&data)
        .expect("Failed to deserialize callback using JSON");
    let b: Result<String, PromiseError> = match ::near_sdk::env::promise_result(1u64) {
        ::near_sdk::PromiseResult::Successful(data) => ::std::result::Result::Ok(
            ::near_sdk::serde_json::from_slice(&data)
                .expect("Failed to deserialize callback using JSON"),
        ),
        ::near_sdk::PromiseResult::Failed => {
            ::std::result::Result::Err(::near_sdk::PromiseError::Failed)
        }
    };
    let c: Result<u8, PromiseError> = match ::near_sdk::env::promise_result(2u64) {
        ::near_sdk::PromiseResult::Successful(data) => ::std::result::Result::Ok(
            ::near_sdk::serde_json::from_slice(&data)
                .expect("Failed to deserialize callback using JSON"),
        ),
        ::near_sdk::PromiseResult::Failed => {
            ::std::result::Result::Err(::near_sdk::PromiseError::Failed)
        }
    };
    let d: Result<(), PromiseError> = match ::near_sdk::env::promise_result(3u64) {
        ::near_sdk::PromiseResult::Successful(data) if data.is_empty() => {
            ::std::result::Result::Ok(())
        }
        ::near_sdk::PromiseResult::Successful(data) => ::std::result::Result::Ok(
            ::near_sdk::serde_json::from_slice(&data)
                .expect("Failed to deserialize callback using JSON"),
        ),
        ::near_sdk::PromiseResult::Failed => {
            ::std::result::Result::Err(::near_sdk::PromiseError::Failed)
        }
    };
    let result = Callback::handle_callbacks(a, b, c, d);
    let result = ::near_sdk::serde_json::to_vec(&result)
        .expect("Failed to serialize the return value using JSON.");
    ::near_sdk::env::value_return(&result);
}
