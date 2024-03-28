use near_sdk::test_utils::get_logs;
use near_sdk::{near, AccountId};

#[near(event_json(standard = "test_standard"))]
pub enum TestEvents<'a, 'b, T>
where
    T: near_sdk::serde::Serialize,
{
    #[event_version("1.0.0")]
    Swap { token_in: AccountId, token_out: AccountId, amount_in: u128, amount_out: u128, test: T },

    #[event_version("2.0.0")]
    StringEvent(String),

    #[event_version("3.0.0")]
    EmptyEvent,

    #[event_version("4.0.0")]
    LifetimeTestA(&'a str),

    #[event_version("5.0.0")]
    LifetimeTestB(&'b str),
}

mod private {
    use super::*;

    #[near(event_json(standard = "another_standard"))]
    pub enum AnotherEvent {
        #[event_version("1.0.0")]
        Test,
    }
}

#[test]
fn test_json_emit() {
    let token_in: AccountId = "wrap.near".parse().unwrap();
    let token_out: AccountId = "test.near".parse().unwrap();
    let amount_in: u128 = 100;
    let amount_out: u128 = 200;
    TestEvents::Swap { token_in, token_out, amount_in, amount_out, test: String::from("tst") }
        .emit();

    TestEvents::StringEvent::<String>(String::from("string")).emit();

    TestEvents::EmptyEvent::<String>.emit();

    TestEvents::LifetimeTestA::<String>("lifetime").emit();

    TestEvents::LifetimeTestB::<String>("lifetime_b").emit();

    private::AnotherEvent::Test.emit();

    let logs = get_logs();

    assert_eq!(
        logs[0],
        r#"EVENT_JSON:{"standard":"test_standard","version":"1.0.0","event":"swap","data":{"token_in":"wrap.near","token_out":"test.near","amount_in":100,"amount_out":200,"test":"tst"}}"#
    );
    assert_eq!(
        logs[1],
        r#"EVENT_JSON:{"standard":"test_standard","version":"2.0.0","event":"string_event","data":"string"}"#
    );
    assert_eq!(
        logs[2],
        r#"EVENT_JSON:{"standard":"test_standard","version":"3.0.0","event":"empty_event"}"#
    );
    assert_eq!(
        logs[3],
        r#"EVENT_JSON:{"standard":"test_standard","version":"4.0.0","event":"lifetime_test_a","data":"lifetime"}"#
    );
    assert_eq!(
        logs[4],
        r#"EVENT_JSON:{"standard":"test_standard","version":"5.0.0","event":"lifetime_test_b","data":"lifetime_b"}"#
    );
    assert_eq!(
        logs[5],
        r#"EVENT_JSON:{"standard":"another_standard","version":"1.0.0","event":"test"}"#
    );
}
