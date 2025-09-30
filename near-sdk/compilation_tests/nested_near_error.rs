//! Method signature uses lifetime.

use near_sdk::near;

#[near(contract_state)]
#[near(contract_metadata(
    version = "39f2d2646f2f60e18ab53337501370dc02a5661c",
    link = "https://github.com/near-examples/nft-tutorial",
    standard(standard = "nep171", version = "1.0.0"),
    standard(standard = "nep177", version = "2.0.0"),
))]
struct CompileFailure {}

fn main() {}
