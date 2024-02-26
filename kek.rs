#[derive(near_sdk :: NearSchema)]
#[derive(near_sdk :: borsh :: BorshSerialize, near_sdk :: borsh ::
BorshDeserialize)] #[borsh(crate = "near_sdk::borsh")]
#[derive(near_sdk :: serde :: Serialize, near_sdk :: serde :: Deserialize)]
#[serde(crate = "near_sdk::serde")] #[abi(borsh, json)] pub struct
Quantity(pub i32) ;
mynear
#[derive(near_sdk :: NearSchema)]
#[derive(near_sdk :: borsh :: BorshSerialize, near_sdk :: borsh ::
BorshDeserialize)] #[borsh(crate = "near_sdk::borsh")] #[abi(borsh)] pub
struct X ;
mynear
#[derive(near_sdk :: NearSchema)]
#[derive(near_sdk :: borsh :: BorshSerialize, near_sdk :: borsh ::
BorshDeserialize)] #[borsh(crate = "near_sdk::borsh")]
#[derive(near_sdk :: serde :: Serialize, near_sdk :: serde :: Deserialize)]
#[serde(crate = "near_sdk::serde")] #[abi(borsh, json)] pub struct
Account(pub HashMap < Asset, Quantity >) ;
mynear
#[derive(near_sdk :: NearSchema)]
#[derive(near_sdk :: borsh :: BorshSerialize, near_sdk :: borsh ::
BorshDeserialize)] #[borsh(crate = "near_sdk::borsh")]
#[derive(near_sdk :: serde :: Serialize, near_sdk :: serde :: Deserialize)]
#[serde(crate = "near_sdk::serde")] #[abi(borsh, json)] pub struct Agent
{ pub account : Account, pub is_alive : bool, }
mynear
#[derive(near_sdk :: NearSchema)]
#[derive(near_sdk :: borsh :: BorshSerialize, near_sdk :: borsh ::
BorshDeserialize)] #[borsh(crate = "near_sdk::borsh")]
#[derive(near_sdk :: serde :: Serialize, near_sdk :: serde :: Deserialize)]
#[serde(crate = "near_sdk::serde")] #[abi(borsh, json)] pub enum Resource
{ Battery, RgbSensor, ThermalSensor, PoseEstimation, }
mynear
#[derive(near_sdk :: NearSchema)]
#[derive(near_sdk :: borsh :: BorshSerialize, near_sdk :: borsh ::
BorshDeserialize)] #[borsh(crate = "near_sdk::borsh")]
#[derive(near_sdk :: serde :: Serialize, near_sdk :: serde :: Deserialize)]
#[serde(crate = "near_sdk::serde")] #[abi(borsh, json)] pub enum Reward
{ Score, Token, Prediction, Currency, Policy, }
mynear
#[derive(near_sdk :: NearSchema)]
#[derive(near_sdk :: borsh :: BorshSerialize, near_sdk :: borsh ::
BorshDeserialize)] #[borsh(crate = "near_sdk::borsh")]
#[derive(near_sdk :: serde :: Serialize, near_sdk :: serde :: Deserialize)]
#[serde(crate = "near_sdk::serde")] #[abi(borsh, json)] pub enum Asset
{ Resource(Resource), Reward(Reward), MissionTime, Trust, }
mynear
#[derive(near_sdk :: NearSchema)]
#[derive(near_sdk :: borsh :: BorshSerialize, near_sdk :: borsh ::
BorshDeserialize)] #[borsh(crate = "near_sdk::borsh")]
#[derive(near_sdk :: serde :: Serialize, near_sdk :: serde :: Deserialize)]
#[serde(crate = "near_sdk::serde")] #[abi(borsh, json)] pub enum Exchange
{ MissionTimeWithResource, MissionTimeWithTrust, }
mynear
#[derive(near_sdk :: NearSchema)]
#[derive(near_sdk :: borsh :: BorshSerialize, near_sdk :: borsh ::
BorshDeserialize)] #[borsh(crate = "near_sdk::borsh")]
#[derive(near_sdk :: serde :: Serialize, near_sdk :: serde :: Deserialize)]
#[serde(crate = "near_sdk::serde")] #[abi(borsh, json)] pub struct
MissionControl
{
    account : Account, agents : HashMap < AccountId, Agent >, rates : HashMap
    < Exchange, Rate >,
}
mynear
impl MissionControlExt
{
    pub fn add_agent(self,) -> :: near_sdk :: Promise
    {
        let __args = :: std :: vec! [] ; :: near_sdk :: Promise ::
        new(self.account_id).function_call_weight(:: std :: string :: String
        :: from("add_agent"), __args, self.deposit, self.static_gas,
        self.gas_weight,)
    } pub fn assets_quantity(self, account_id : AccountId, asset : Asset,) ->
    :: near_sdk :: Promise
    {
        let __args =
        {
            #[derive(:: near_sdk :: serde :: Serialize)]
            #[serde(crate = "::near_sdk::serde")] struct Input < 'nearinput >
            {
                account_id : & 'nearinput AccountId, asset : & 'nearinput
                Asset,
            } let __args = Input
            { account_id : & account_id, asset : & asset, } ; :: near_sdk ::
            serde_json ::
            to_vec(&
            __args).expect("Failed to serialize the cross contract args using JSON.")
        } ; :: near_sdk :: Promise ::
        new(self.account_id).function_call_weight(:: std :: string :: String
        :: from("assets_quantity"), __args, self.deposit, self.static_gas,
        self.gas_weight,)
    } pub fn simulate(self, account_id : AccountId,) -> :: near_sdk :: Promise
    {
        let __args =
        {
            #[derive(:: near_sdk :: serde :: Serialize)]
            #[serde(crate = "::near_sdk::serde")] struct Input < 'nearinput >
            { account_id : & 'nearinput AccountId, } let __args = Input
            { account_id : & account_id, } ; :: near_sdk :: serde_json ::
            to_vec(&
            __args).expect("Failed to serialize the cross contract args using JSON.")
        } ; :: near_sdk :: Promise ::
        new(self.account_id).function_call_weight(:: std :: string :: String
        :: from("simulate"), __args, self.deposit, self.static_gas,
        self.gas_weight,)
    }
}
process_impl_block
#[derive(near_sdk :: NearSchema)]
#[derive(near_sdk :: borsh :: BorshSerialize, near_sdk :: borsh ::
BorshDeserialize)] #[borsh(crate = "near_sdk::borsh")]
#[derive(near_sdk :: serde :: Serialize, near_sdk :: serde :: Deserialize)]
#[serde(crate = "near_sdk::serde")] #[abi(borsh, json)] pub struct Rate
{
    pub credit : HashMap < Asset, Quantity >, pub debit : HashMap < Asset,
    Quantity >,
}
mynear