use near_sdk::near;

type MyResult = Result<u32, &'static str>;

#[derive(Default)]
#[near(contract_state)]
struct Contract {
    value: u32,
}

#[near]
impl Contract {
    #[handle_result(aliased)]
    pub fn fun(&self) -> MyResult {
        Err("error")
    }
}

fn main() {}
