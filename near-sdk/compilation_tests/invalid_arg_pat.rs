//! Method with non-deserializable argument type.
//This tests also checks whether argument errors gets combined or not.
//faulty_method checks a combination of serialiser and type not not supported
//faulty_method1 checks a combination of serialiser and only Identity pattern allowed.
//It is not possible to check Identity pattern and Type not supported together.
use near_sdk::near;
use near_sdk::PanicOnDefault;

#[near(contract_state)]
#[derive(PanicOnDefault)]
struct Storage {}

#[near]
impl Storage {
    pub fn faulty_method(&mut self, #[serializer(SomeNonExistentSerializer)] _a: *mut u32) {}
    pub fn faulty_method1(&mut self, #[serializer(SomeNonExistentSerializer)] (a, b): (u8, u32)) {}
}

fn main() {}
