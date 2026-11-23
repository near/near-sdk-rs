use super::*;

#[test]
#[should_panic(expected = "PANIC!")]
fn test_panic() {
    let mut contract = TestContract::new();
    contract.test_panic_macro();
}
