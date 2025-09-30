use near_sdk::near;

#[near(contract_state, contract_metadata(
    standard(standard = "nep330", version = "1.1.0"),
    standard(standard = "nep171", version = "1.0.0"),
    standard(standard = "nep177", version = "2.0.0"),
))]
struct Contract {}

#[near]
impl Contract {}

fn main() {}
