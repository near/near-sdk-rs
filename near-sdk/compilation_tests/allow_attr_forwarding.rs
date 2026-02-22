//! Contract that verifies #[allow] attributes on methods are respected
//! through the #[near] macro expansion, including the generated ext wrappers.
//! See https://github.com/near/near-sdk-rs/issues/1505

use near_sdk::near;

#[near(contract_state)]
#[derive(Default)]
struct Contract {
    value: u32,
}

#[near]
impl Contract {
    #[allow(clippy::too_many_arguments)]
    pub fn many_args(
        &self,
        arg1: u32,
        arg2: u32,
        arg3: u32,
        arg4: u32,
        arg5: u32,
        arg6: u32,
        arg7: u32,
        arg8: u32,
    ) -> u32 {
        arg1 + arg2 + arg3 + arg4 + arg5 + arg6 + arg7 + arg8
    }
}

fn main() {}
