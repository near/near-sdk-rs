use near_sdk::{Promise, near};

#[derive(Debug, PartialEq, Eq)]
#[near(serializers=[borsh, json])]
pub struct Pair(u32, u32);

#[derive(Default)]
#[near(serializers=[borsh, json], contract_state)]
pub struct Adder {}

#[near]
impl Adder {
    pub fn a(&self) -> Pair {
        Pair(1, 1)
    }

    pub fn b(&self) -> Pair {
        Pair(2, 3)
    }

    pub fn c(&self) -> Pair {
        Pair(5, 8)
    }

    pub fn d(&self) -> Pair {
        Pair(13, 21)
    }

    pub fn add_callback_vec(&self, #[callback_vec] elements: Vec<Pair>) -> Pair {
        let start = Pair(0, 0);
        elements.iter().fold(start, |acc, el| Pair(acc.0 + el.0, acc.1 + el.1))
    }

    /// Call functions a, b, and c, d,  asynchronously and handle results with `add_callback_vec`.
    pub fn call_all() -> Promise {
        Self::ext_self()
            .a()
            .and(Self::ext_self().b())
            .and(Self::ext_self().c())
            .and(Self::ext_self().d())
            .then(Self::ext_self().add_callback_vec())
    }
}
