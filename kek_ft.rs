pub trait ExtStatusMessage
{
    fn set_status(& mut self, message : String) ; fn
    get_status(& self, account_id : AccountId) -> Option < String > ;
} 
pub mod ext_status_message
{
    use super :: * ; #[must_use] pub struct ExtStatusMessageExt
    {
        pub(crate) account_id : :: near_sdk :: AccountId, pub(crate) deposit :
        :: near_sdk :: NearToken, pub(crate) static_gas : :: near_sdk :: Gas,
        pub(crate) gas_weight : :: near_sdk :: GasWeight,
    } impl ExtStatusMessageExt
    {
        pub fn
        with_attached_deposit(mut self, amount : :: near_sdk :: NearToken) ->
        Self { self.deposit = amount ; self } pub fn
        with_static_gas(mut self, static_gas : :: near_sdk :: Gas) -> Self
        { self.static_gas = static_gas ; self } pub fn
        with_unused_gas_weight(mut self, gas_weight : u64) -> Self
        { self.gas_weight = :: near_sdk :: GasWeight(gas_weight) ; self }
    }
    #[doc =
    r" API for calling this contract's functions in a subsequent execution."]
    pub fn ext(account_id : :: near_sdk :: AccountId) -> ExtStatusMessageExt
    {
        ExtStatusMessageExt
        {
            account_id, deposit : :: near_sdk :: NearToken :: from_near(0),
            static_gas : :: near_sdk :: Gas :: from_gas(0), gas_weight : ::
            near_sdk :: GasWeight :: default(),
        }
    } impl ExtStatusMessageExt
    {
        pub fn set_status(self, message : String,) -> :: near_sdk :: Promise
        {
            let __args =
            {
                #[derive(:: near_sdk :: serde :: Serialize)]
                #[serde(crate = "::near_sdk::serde")] struct Input <
                'nearinput > { message : & 'nearinput String, } let __args =
                Input { message : & message, } ; :: near_sdk :: serde_json ::
                to_vec(&
                __args).expect("Failed to serialize the cross contract args using JSON.")
            } ; :: near_sdk :: Promise ::
            new(self.account_id).function_call_weight(:: std :: string ::
            String :: from("set_status"), __args, self.deposit,
            self.static_gas, self.gas_weight,)
        } pub fn get_status(self, account_id : AccountId,) -> :: near_sdk ::
        Promise
        {
            let __args =
            {
                #[derive(:: near_sdk :: serde :: Serialize)]
                #[serde(crate = "::near_sdk::serde")] struct Input <
                'nearinput > { account_id : & 'nearinput AccountId, } let
                __args = Input { account_id : & account_id, } ; :: near_sdk ::
                serde_json ::
                to_vec(&
                __args).expect("Failed to serialize the cross contract args using JSON.")
            } ; :: near_sdk :: Promise ::
            new(self.account_id).function_call_weight(:: std :: string ::
            String :: from("get_status"), __args, self.deposit,
            self.static_gas, self.gas_weight,)
        }
    }
}

//////////////////-------------------------------------------------





impl ContractExt {
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
impl Contract {
    pub fn contract_source_metadata() {
        near_sdk::env::value_return(CONTRACT_SOURCE_METADATA.as_bytes())
    }
}
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn contract_source_metadata() {
    ::near_sdk::env::setup_panic_hook();
    Contract::contract_source_metadata();
}
#[derive(
    :: near_sdk :: borsh :: BorshSerialize,
    :: near_sdk :: borsh ::
BorshDeserialize,
)]
#[borsh(crate = ":: near_sdk :: borsh")]
pub struct Contract {
    token: FungibleToken,
    metadata: LazyOption<FungibleTokenMetadata>,
}
#[must_use]
pub struct ContractExt {
    pub(crate) account_id: ::near_sdk::AccountId,
    pub(crate) deposit: ::near_sdk::NearToken,
    pub(crate) static_gas: ::near_sdk::Gas,
    pub(crate) gas_weight: ::near_sdk::GasWeight,
}
impl ContractExt {
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
impl Contract {
    #[doc = r" API for calling this contract's functions in a subsequent execution."]
    pub fn ext(account_id: ::near_sdk::AccountId) -> ContractExt {
        ContractExt {
            account_id,
            deposit: ::near_sdk::NearToken::from_near(0),
            static_gas: ::near_sdk::Gas::from_gas(0),
            gas_weight: ::near_sdk::GasWeight::default(),
        }
    }
}
pub const CONTRACT_SOURCE_METADATA : & 'static str =
"{\"version\":\"1.1.0\",\"link\":null,\"standards\":[{\"standard\":\"nep330\",\"version\":\"1.1.0\"}]}"
;
impl ContractExt {
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
impl Contract {
    pub fn contract_source_metadata() {
        near_sdk::env::value_return(CONTRACT_SOURCE_METADATA.as_bytes())
    }
}
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn contract_source_metadata() {
    ::near_sdk::env::setup_panic_hook();
    Contract::contract_source_metadata();
}
impl ContractExt {
    pub fn new_default_meta(self, owner_id: AccountId, total_supply: U128) -> ::near_sdk::Promise {
        let __args = {
            #[derive(:: near_sdk :: serde :: Serialize)]
            #[serde(crate = "::near_sdk::serde")]
            struct Input<'nearinput> {
                owner_id: &'nearinput AccountId,
                total_supply: &'nearinput U128,
            }
            let __args = Input { owner_id: &owner_id, total_supply: &total_supply };
            ::near_sdk::serde_json::to_vec(&__args)
                .expect("Failed to serialize the cross contract args using JSON.")
        };
        ::near_sdk::Promise::new(self.account_id).function_call_weight(
            ::std::string::String::from("new_default_meta"),
            __args,
            self.deposit,
            self.static_gas,
            self.gas_weight,
        )
    }
    pub fn new(
        self,
        owner_id: AccountId,
        total_supply: U128,
        metadata: FungibleTokenMetadata,
    ) -> ::near_sdk::Promise {
        let __args = {
            #[derive(:: near_sdk :: serde :: Serialize)]
            #[serde(crate = "::near_sdk::serde")]
            struct Input<'nearinput> {
                owner_id: &'nearinput AccountId,
                total_supply: &'nearinput U128,
                metadata: &'nearinput FungibleTokenMetadata,
            }
            let __args =
                Input { owner_id: &owner_id, total_supply: &total_supply, metadata: &metadata };
            ::near_sdk::serde_json::to_vec(&__args)
                .expect("Failed to serialize the cross contract args using JSON.")
        };
        ::near_sdk::Promise::new(self.account_id).function_call_weight(
            ::std::string::String::from("new"),
            __args,
            self.deposit,
            self.static_gas,
            self.gas_weight,
        )
    }
}
impl Contract {
    #[doc = " Initializes the contract with the given total supply owned by the given `owner_id` with"]
    #[doc = " default metadata (for example purposes only)."]
    pub fn new_default_meta(owner_id: AccountId, total_supply: U128) -> Contract {
        Self::new(
            owner_id,
            total_supply,
            FungibleTokenMetadata {
                spec: FT_METADATA_SPEC.to_string(),
                name: "Example NEAR fungible token".to_string(),
                symbol: "EXAMPLE".to_string(),
                icon: Some(DATA_IMAGE_SVG_NEAR_ICON.to_string()),
                reference: None,
                reference_hash: None,
                decimals: 24,
            },
        )
    }
    #[doc = " Initializes the contract with the given total supply owned by the given `owner_id` with"]
    #[doc = " the given fungible token metadata."]
    pub fn new(
        owner_id: AccountId,
        total_supply: U128,
        metadata: FungibleTokenMetadata,
    ) -> Contract {
        require!(!env::state_exists(), "Already initialized");
        metadata.assert_valid();
        let mut this = Self {
            token: FungibleToken::new(StorageKey::FungibleToken),
            metadata: LazyOption::new(StorageKey::Metadata, Some(&metadata)),
        };
        this.token.internal_register_account(&owner_id);
        this.token.internal_deposit(&owner_id, total_supply.into());
        near_contract_standards::fungible_token::events::FtMint {
            owner_id: &owner_id,
            amount: total_supply,
            memo: Some("new tokens are minted"),
        }
        .emit();
        this
    }
}
#[doc = " Initializes the contract with the given total supply owned by the given `owner_id` with"]
#[doc = " default metadata (for example purposes only)."]
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn new_default_meta() {
    ::near_sdk::env::setup_panic_hook();
    if ::near_sdk::env::attached_deposit().as_yoctonear() != 0 {
        ::near_sdk::env::panic_str("Method new_default_meta doesn't accept deposit");
    }
    #[derive(:: near_sdk :: serde :: Deserialize)]
    #[serde(crate = "::near_sdk::serde")]
    struct Input {
        owner_id: AccountId,
        total_supply: U128,
    }
    let Input { owner_id, total_supply }: Input = ::near_sdk::serde_json::from_slice(
        &::near_sdk::env::input().expect("Expected input since method has arguments."),
    )
    .expect("Failed to deserialize input from JSON.");
    if ::near_sdk::env::state_exists() {
        ::near_sdk::env::panic_str("The contract has already been initialized");
    }
    let contract = Contract::new_default_meta(owner_id, total_supply);
    ::near_sdk::env::state_write(&contract);
}
#[doc = " Initializes the contract with the given total supply owned by the given `owner_id` with"]
#[doc = " the given fungible token metadata."]
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn new() {
    ::near_sdk::env::setup_panic_hook();
    if ::near_sdk::env::attached_deposit().as_yoctonear() != 0 {
        ::near_sdk::env::panic_str("Method new doesn't accept deposit");
    }
    #[derive(:: near_sdk :: serde :: Deserialize)]
    #[serde(crate = "::near_sdk::serde")]
    struct Input {
        owner_id: AccountId,
        total_supply: U128,
        metadata: FungibleTokenMetadata,
    }
    let Input { owner_id, total_supply, metadata }: Input = ::near_sdk::serde_json::from_slice(
        &::near_sdk::env::input().expect("Expected input since method has arguments."),
    )
    .expect("Failed to deserialize input from JSON.");
    if ::near_sdk::env::state_exists() {
        ::near_sdk::env::panic_str("The contract has already been initialized");
    }
    let contract = Contract::new(owner_id, total_supply, metadata);
    ::near_sdk::env::state_write(&contract);
}
impl ContractExt {
    pub fn ft_transfer(
        self,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
    ) -> ::near_sdk::Promise {
        let __args = {
            #[derive(:: near_sdk :: serde :: Serialize)]
            #[serde(crate = "::near_sdk::serde")]
            struct Input<'nearinput> {
                receiver_id: &'nearinput AccountId,
                amount: &'nearinput U128,
                memo: &'nearinput Option<String>,
            }
            let __args = Input { receiver_id: &receiver_id, amount: &amount, memo: &memo };
            ::near_sdk::serde_json::to_vec(&__args)
                .expect("Failed to serialize the cross contract args using JSON.")
        };
        ::near_sdk::Promise::new(self.account_id).function_call_weight(
            ::std::string::String::from("ft_transfer"),
            __args,
            self.deposit,
            self.static_gas,
            self.gas_weight,
        )
    }
    pub fn ft_transfer_call(
        self,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
        msg: String,
    ) -> ::near_sdk::Promise {
        let __args = {
            #[derive(:: near_sdk :: serde :: Serialize)]
            #[serde(crate = "::near_sdk::serde")]
            struct Input<'nearinput> {
                receiver_id: &'nearinput AccountId,
                amount: &'nearinput U128,
                memo: &'nearinput Option<String>,
                msg: &'nearinput String,
            }
            let __args =
                Input { receiver_id: &receiver_id, amount: &amount, memo: &memo, msg: &msg };
            ::near_sdk::serde_json::to_vec(&__args)
                .expect("Failed to serialize the cross contract args using JSON.")
        };
        ::near_sdk::Promise::new(self.account_id).function_call_weight(
            ::std::string::String::from("ft_transfer_call"),
            __args,
            self.deposit,
            self.static_gas,
            self.gas_weight,
        )
    }
    pub fn ft_total_supply(self) -> ::near_sdk::Promise {
        let __args = ::std::vec![];
        ::near_sdk::Promise::new(self.account_id).function_call_weight(
            ::std::string::String::from("ft_total_supply"),
            __args,
            self.deposit,
            self.static_gas,
            self.gas_weight,
        )
    }
    pub fn ft_balance_of(self, account_id: AccountId) -> ::near_sdk::Promise {
        let __args = {
            #[derive(:: near_sdk :: serde :: Serialize)]
            #[serde(crate = "::near_sdk::serde")]
            struct Input<'nearinput> {
                account_id: &'nearinput AccountId,
            }
            let __args = Input { account_id: &account_id };
            ::near_sdk::serde_json::to_vec(&__args)
                .expect("Failed to serialize the cross contract args using JSON.")
        };
        ::near_sdk::Promise::new(self.account_id).function_call_weight(
            ::std::string::String::from("ft_balance_of"),
            __args,
            self.deposit,
            self.static_gas,
            self.gas_weight,
        )
    }
}
impl FungibleTokenCore for Contract {
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>) {
        self.token.ft_transfer(receiver_id, amount, memo)
    }
    fn ft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<U128> {
        self.token.ft_transfer_call(receiver_id, amount, memo, msg)
    }
    fn ft_total_supply(&self) -> U128 {
        self.token.ft_total_supply()
    }
    fn ft_balance_of(&self, account_id: AccountId) -> U128 {
        self.token.ft_balance_of(account_id)
    }
}
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn ft_transfer() {
    ::near_sdk::env::setup_panic_hook();
    #[derive(:: near_sdk :: serde :: Deserialize)]
    #[serde(crate = "::near_sdk::serde")]
    struct Input {
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
    }
    let Input { receiver_id, amount, memo }: Input = ::near_sdk::serde_json::from_slice(
        &::near_sdk::env::input().expect("Expected input since method has arguments."),
    )
    .expect("Failed to deserialize input from JSON.");
    let mut contract: Contract = ::near_sdk::env::state_read().unwrap_or_default();
    contract.ft_transfer(receiver_id, amount, memo);
    ::near_sdk::env::state_write(&contract);
}
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn ft_transfer_call() {
    ::near_sdk::env::setup_panic_hook();
    #[derive(:: near_sdk :: serde :: Deserialize)]
    #[serde(crate = "::near_sdk::serde")]
    struct Input {
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
        msg: String,
    }
    let Input { receiver_id, amount, memo, msg }: Input = ::near_sdk::serde_json::from_slice(
        &::near_sdk::env::input().expect("Expected input since method has arguments."),
    )
    .expect("Failed to deserialize input from JSON.");
    let mut contract: Contract = ::near_sdk::env::state_read().unwrap_or_default();
    let result = contract.ft_transfer_call(receiver_id, amount, memo, msg);
    let result = ::near_sdk::serde_json::to_vec(&result)
        .expect("Failed to serialize the return value using JSON.");
    ::near_sdk::env::value_return(&result);
    ::near_sdk::env::state_write(&contract);
}
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn ft_total_supply() {
    ::near_sdk::env::setup_panic_hook();
    let contract: Contract = ::near_sdk::env::state_read().unwrap_or_default();
    let result = contract.ft_total_supply();
    let result = ::near_sdk::serde_json::to_vec(&result)
        .expect("Failed to serialize the return value using JSON.");
    ::near_sdk::env::value_return(&result);
}
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn ft_balance_of() {
    ::near_sdk::env::setup_panic_hook();
    #[derive(:: near_sdk :: serde :: Deserialize)]
    #[serde(crate = "::near_sdk::serde")]
    struct Input {
        account_id: AccountId,
    }
    let Input { account_id }: Input = ::near_sdk::serde_json::from_slice(
        &::near_sdk::env::input().expect("Expected input since method has arguments."),
    )
    .expect("Failed to deserialize input from JSON.");
    let contract: Contract = ::near_sdk::env::state_read().unwrap_or_default();
    let result = contract.ft_balance_of(account_id);
    let result = ::near_sdk::serde_json::to_vec(&result)
        .expect("Failed to serialize the return value using JSON.");
    ::near_sdk::env::value_return(&result);
}
impl ContractExt {
    pub fn ft_resolve_transfer(
        self,
        sender_id: AccountId,
        receiver_id: AccountId,
        amount: U128,
    ) -> ::near_sdk::Promise {
        let __args = {
            #[derive(:: near_sdk :: serde :: Serialize)]
            #[serde(crate = "::near_sdk::serde")]
            struct Input<'nearinput> {
                sender_id: &'nearinput AccountId,
                receiver_id: &'nearinput AccountId,
                amount: &'nearinput U128,
            }
            let __args =
                Input { sender_id: &sender_id, receiver_id: &receiver_id, amount: &amount };
            ::near_sdk::serde_json::to_vec(&__args)
                .expect("Failed to serialize the cross contract args using JSON.")
        };
        ::near_sdk::Promise::new(self.account_id).function_call_weight(
            ::std::string::String::from("ft_resolve_transfer"),
            __args,
            self.deposit,
            self.static_gas,
            self.gas_weight,
        )
    }
}
impl FungibleTokenResolver for Contract {
    fn ft_resolve_transfer(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        amount: U128,
    ) -> U128 {
        let (used_amount, burned_amount) =
            self.token.internal_ft_resolve_transfer(&sender_id, receiver_id, amount);
        if burned_amount > 0 {
            log!("Account @{} burned {}", sender_id, burned_amount);
        }
        used_amount.into()
    }
}
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn ft_resolve_transfer() {
    ::near_sdk::env::setup_panic_hook();
    if ::near_sdk::env::current_account_id() != ::near_sdk::env::predecessor_account_id() {
        ::near_sdk::env::panic_str("Method ft_resolve_transfer is private");
    }
    if ::near_sdk::env::attached_deposit().as_yoctonear() != 0 {
        ::near_sdk::env::panic_str("Method ft_resolve_transfer doesn't accept deposit");
    }
    #[derive(:: near_sdk :: serde :: Deserialize)]
    #[serde(crate = "::near_sdk::serde")]
    struct Input {
        sender_id: AccountId,
        receiver_id: AccountId,
        amount: U128,
    }
    let Input { sender_id, receiver_id, amount }: Input = ::near_sdk::serde_json::from_slice(
        &::near_sdk::env::input().expect("Expected input since method has arguments."),
    )
    .expect("Failed to deserialize input from JSON.");
    let mut contract: Contract = ::near_sdk::env::state_read().unwrap_or_default();
    let result = contract.ft_resolve_transfer(sender_id, receiver_id, amount);
    let result = ::near_sdk::serde_json::to_vec(&result)
        .expect("Failed to serialize the return value using JSON.");
    ::near_sdk::env::value_return(&result);
    ::near_sdk::env::state_write(&contract);
}
impl ContractExt {
    pub fn storage_deposit(
        self,
        account_id: Option<AccountId>,
        registration_only: Option<bool>,
    ) -> ::near_sdk::Promise {
        let __args = {
            #[derive(:: near_sdk :: serde :: Serialize)]
            #[serde(crate = "::near_sdk::serde")]
            struct Input<'nearinput> {
                account_id: &'nearinput Option<AccountId>,
                registration_only: &'nearinput Option<bool>,
            }
            let __args = Input { account_id: &account_id, registration_only: &registration_only };
            ::near_sdk::serde_json::to_vec(&__args)
                .expect("Failed to serialize the cross contract args using JSON.")
        };
        ::near_sdk::Promise::new(self.account_id).function_call_weight(
            ::std::string::String::from("storage_deposit"),
            __args,
            self.deposit,
            self.static_gas,
            self.gas_weight,
        )
    }
    pub fn storage_withdraw(self, amount: Option<NearToken>) -> ::near_sdk::Promise {
        let __args = {
            #[derive(:: near_sdk :: serde :: Serialize)]
            #[serde(crate = "::near_sdk::serde")]
            struct Input<'nearinput> {
                amount: &'nearinput Option<NearToken>,
            }
            let __args = Input { amount: &amount };
            ::near_sdk::serde_json::to_vec(&__args)
                .expect("Failed to serialize the cross contract args using JSON.")
        };
        ::near_sdk::Promise::new(self.account_id).function_call_weight(
            ::std::string::String::from("storage_withdraw"),
            __args,
            self.deposit,
            self.static_gas,
            self.gas_weight,
        )
    }
    pub fn storage_unregister(self, force: Option<bool>) -> ::near_sdk::Promise {
        let __args = {
            #[derive(:: near_sdk :: serde :: Serialize)]
            #[serde(crate = "::near_sdk::serde")]
            struct Input<'nearinput> {
                force: &'nearinput Option<bool>,
            }
            let __args = Input { force: &force };
            ::near_sdk::serde_json::to_vec(&__args)
                .expect("Failed to serialize the cross contract args using JSON.")
        };
        ::near_sdk::Promise::new(self.account_id).function_call_weight(
            ::std::string::String::from("storage_unregister"),
            __args,
            self.deposit,
            self.static_gas,
            self.gas_weight,
        )
    }
    pub fn storage_balance_bounds(self) -> ::near_sdk::Promise {
        let __args = ::std::vec![];
        ::near_sdk::Promise::new(self.account_id).function_call_weight(
            ::std::string::String::from("storage_balance_bounds"),
            __args,
            self.deposit,
            self.static_gas,
            self.gas_weight,
        )
    }
    pub fn storage_balance_of(self, account_id: AccountId) -> ::near_sdk::Promise {
        let __args = {
            #[derive(:: near_sdk :: serde :: Serialize)]
            #[serde(crate = "::near_sdk::serde")]
            struct Input<'nearinput> {
                account_id: &'nearinput AccountId,
            }
            let __args = Input { account_id: &account_id };
            ::near_sdk::serde_json::to_vec(&__args)
                .expect("Failed to serialize the cross contract args using JSON.")
        };
        ::near_sdk::Promise::new(self.account_id).function_call_weight(
            ::std::string::String::from("storage_balance_of"),
            __args,
            self.deposit,
            self.static_gas,
            self.gas_weight,
        )
    }
}
impl StorageManagement for Contract {
    fn storage_deposit(
        &mut self,
        account_id: Option<AccountId>,
        registration_only: Option<bool>,
    ) -> StorageBalance {
        self.token.storage_deposit(account_id, registration_only)
    }
    fn storage_withdraw(&mut self, amount: Option<NearToken>) -> StorageBalance {
        self.token.storage_withdraw(amount)
    }
    fn storage_unregister(&mut self, force: Option<bool>) -> bool {
        #[allow(unused_variables)]
        if let Some((account_id, balance)) = self.token.internal_storage_unregister(force) {
            log!("Closed @{} with {}", account_id, balance);
            true
        } else {
            false
        }
    }
    fn storage_balance_bounds(&self) -> StorageBalanceBounds {
        self.token.storage_balance_bounds()
    }
    fn storage_balance_of(&self, account_id: AccountId) -> Option<StorageBalance> {
        self.token.storage_balance_of(account_id)
    }
}
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn storage_deposit() {
    ::near_sdk::env::setup_panic_hook();
    #[derive(:: near_sdk :: serde :: Deserialize)]
    #[serde(crate = "::near_sdk::serde")]
    struct Input {
        account_id: Option<AccountId>,
        registration_only: Option<bool>,
    }
    let Input { account_id, registration_only }: Input = ::near_sdk::serde_json::from_slice(
        &::near_sdk::env::input().expect("Expected input since method has arguments."),
    )
    .expect("Failed to deserialize input from JSON.");
    let mut contract: Contract = ::near_sdk::env::state_read().unwrap_or_default();
    let result = contract.storage_deposit(account_id, registration_only);
    let result = ::near_sdk::serde_json::to_vec(&result)
        .expect("Failed to serialize the return value using JSON.");
    ::near_sdk::env::value_return(&result);
    ::near_sdk::env::state_write(&contract);
}
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn storage_withdraw() {
    ::near_sdk::env::setup_panic_hook();
    #[derive(:: near_sdk :: serde :: Deserialize)]
    #[serde(crate = "::near_sdk::serde")]
    struct Input {
        amount: Option<NearToken>,
    }
    let Input { amount }: Input = ::near_sdk::serde_json::from_slice(
        &::near_sdk::env::input().expect("Expected input since method has arguments."),
    )
    .expect("Failed to deserialize input from JSON.");
    let mut contract: Contract = ::near_sdk::env::state_read().unwrap_or_default();
    let result = contract.storage_withdraw(amount);
    let result = ::near_sdk::serde_json::to_vec(&result)
        .expect("Failed to serialize the return value using JSON.");
    ::near_sdk::env::value_return(&result);
    ::near_sdk::env::state_write(&contract);
}
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn storage_unregister() {
    ::near_sdk::env::setup_panic_hook();
    #[derive(:: near_sdk :: serde :: Deserialize)]
    #[serde(crate = "::near_sdk::serde")]
    struct Input {
        force: Option<bool>,
    }
    let Input { force }: Input = ::near_sdk::serde_json::from_slice(
        &::near_sdk::env::input().expect("Expected input since method has arguments."),
    )
    .expect("Failed to deserialize input from JSON.");
    let mut contract: Contract = ::near_sdk::env::state_read().unwrap_or_default();
    let result = contract.storage_unregister(force);
    let result = ::near_sdk::serde_json::to_vec(&result)
        .expect("Failed to serialize the return value using JSON.");
    ::near_sdk::env::value_return(&result);
    ::near_sdk::env::state_write(&contract);
}
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn storage_balance_bounds() {
    ::near_sdk::env::setup_panic_hook();
    let contract: Contract = ::near_sdk::env::state_read().unwrap_or_default();
    let result = contract.storage_balance_bounds();
    let result = ::near_sdk::serde_json::to_vec(&result)
        .expect("Failed to serialize the return value using JSON.");
    ::near_sdk::env::value_return(&result);
}
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn storage_balance_of() {
    ::near_sdk::env::setup_panic_hook();
    #[derive(:: near_sdk :: serde :: Deserialize)]
    #[serde(crate = "::near_sdk::serde")]
    struct Input {
        account_id: AccountId,
    }
    let Input { account_id }: Input = ::near_sdk::serde_json::from_slice(
        &::near_sdk::env::input().expect("Expected input since method has arguments."),
    )
    .expect("Failed to deserialize input from JSON.");
    let contract: Contract = ::near_sdk::env::state_read().unwrap_or_default();
    let result = contract.storage_balance_of(account_id);
    let result = ::near_sdk::serde_json::to_vec(&result)
        .expect("Failed to serialize the return value using JSON.");
    ::near_sdk::env::value_return(&result);
}
impl ContractExt {
    pub fn ft_metadata(self) -> ::near_sdk::Promise {
        let __args = ::std::vec![];
        ::near_sdk::Promise::new(self.account_id).function_call_weight(
            ::std::string::String::from("ft_metadata"),
            __args,
            self.deposit,
            self.static_gas,
            self.gas_weight,
        )
    }
}
impl FungibleTokenMetadataProvider for Contract {
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        self.metadata.get().unwrap()
    }
}
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn ft_metadata() {
    ::near_sdk::env::setup_panic_hook();
    let contract: Contract = ::near_sdk::env::state_read().unwrap_or_default();
    let result = contract.ft_metadata();
    let result = ::near_sdk::serde_json::to_vec(&result)
        .expect("Failed to serialize the return value using JSON.");
    ::near_sdk::env::value_return(&result);
}
