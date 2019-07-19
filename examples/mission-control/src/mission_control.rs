use crate::account::*;
use crate::agent::Agent;
use crate::asset::*;
use crate::rate::*;
use near_bindgen::{near_bindgen, CONTEXT};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type AccountId = Vec<u8>;

#[near_bindgen]
#[derive(Serialize, Deserialize)]
pub struct MissionControl {
    account: Account,
    agents: HashMap<AccountId, Agent>,
    rates: HashMap<Exchange, Rate>,
}

#[near_bindgen]
impl MissionControl {
    pub fn add_agent(&mut self) {
        let account_id = CONTEXT.originator_id();
        self.agents.insert(account_id, Agent { account: agent_default(), is_alive: true });
    }

    pub fn assets_quantity(&self, account_id: String, asset: Asset) -> Option<Quantity> {
        let account_id = account_id.into_bytes();
        self.agents.get(&account_id).and_then(|agent| (agent.account.0).get(&asset).cloned())
    }

    pub fn simulate(&mut self, account_id: String) -> Option<bool> {
        let account_id = account_id.into_bytes();
        let Self { agents, rates, account } = self;
        agents.get_mut(&account_id).map(|agent| {
            agent.simulate(rates, account);
            agent.is_alive
        })
    }
}

impl Default for MissionControl {
    fn default() -> Self {
        Self { account: mission_default(), agents: Default::default(), rates: rates_default() }
    }
}

fn mission_default() -> Account {
    Account(hashmap![
        Asset::MissionTime => Quantity(1000000),
    ])
}

fn agent_default() -> Account {
    Account(hashmap![
        Asset::MissionTime => Quantity(1),
        Asset::Trust => Quantity(10000),
        Asset::Resource(Resource::Battery) => Quantity(10000),
        Asset::Resource(Resource::RgbSensor) => Quantity(10000),
        Asset::Resource(Resource::ThermalSensor) => Quantity(10000),
        Asset::Resource(Resource::PoseEstimation) => Quantity(10000),
    ])
}

fn rates_default() -> HashMap<Exchange, Rate> {
    hashmap![
        Exchange::MissionTimeWithResource =>
        Rate {
            credit: hashmap![Asset::MissionTime => Quantity(1)],
            debit: hashmap![
                Asset::Resource(Resource::Battery) => Quantity(20),
                Asset::Resource(Resource::ThermalSensor) => Quantity(9),
                Asset::Resource(Resource::RgbSensor) => Quantity(3),
                Asset::Resource(Resource::PoseEstimation) => Quantity(1),
            ],
        },
        Exchange::MissionTimeWithTrust =>
        Rate {
            credit: hashmap![Asset::MissionTime => Quantity(1)],
            debit: hashmap![Asset::Trust => Quantity(1)],
        },
    ]
}

#[cfg(feature = "env_test")]
#[cfg(test)]
mod tests {
    use crate::mission_control::MissionControl;
    use near_bindgen::MockedContext;
    use near_bindgen::CONTEXT;
    use crate::asset::Asset::MissionTime;
    use crate::account::Quantity;

    #[test]
    fn add_agent() {
        CONTEXT.set(Box::new(MockedContext::new()));
        let account_id = "alice";
        CONTEXT.as_mock().set_originator_id(account_id.as_bytes().to_vec());
        let mut contract = MissionControl::default();
        contract.add_agent();
        assert_eq!(Some(true), contract.simulate(account_id.to_owned()));
        assert_eq!(Some(Quantity(2)), contract.assets_quantity(account_id.to_owned(), MissionTime));
    }
}
