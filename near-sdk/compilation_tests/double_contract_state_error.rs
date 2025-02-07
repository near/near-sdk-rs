use near_sdk::near;

#[near(contract_state)]
pub struct Contract {}

pub mod mod1 {
    use near_sdk::near;

    #[near(contract_state)]
    struct Contract {}
}

fn main() {}
