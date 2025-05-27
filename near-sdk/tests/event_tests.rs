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

    {
        let log0: serde_json::Value =
            serde_json::from_str(&logs[0].strip_prefix("EVENT_JSON:").unwrap()).unwrap();

        assert_eq!(log0.as_object().unwrap().len(), 4);
        assert_eq!(log0.get("standard").unwrap(), "test_standard");
        assert_eq!(log0.get("version").unwrap(), "1.0.0");
        assert_eq!(log0.get("event").unwrap(), "swap");

        let data0 = log0.get("data").unwrap();
        assert_eq!(data0.as_object().unwrap().len(), 5);
        assert_eq!(data0.get("token_in").unwrap(), "wrap.near");
        assert_eq!(data0.get("token_out").unwrap(), "test.near");
        assert_eq!(data0.get("amount_in").unwrap(), 100);
        assert_eq!(data0.get("amount_out").unwrap(), 200);
        assert_eq!(data0.get("test").unwrap(), "tst");
    }
    {
        let log1: serde_json::Value =
            serde_json::from_str(&logs[1].strip_prefix("EVENT_JSON:").unwrap()).unwrap();
        assert_eq!(log1.as_object().unwrap().len(), 4);
        assert_eq!(log1.get("standard").unwrap(), "test_standard");
        assert_eq!(log1.get("version").unwrap(), "2.0.0");
        assert_eq!(log1.get("event").unwrap(), "string_event");
        assert_eq!(log1.get("data").unwrap(), "string");
    }
    {
        let log2: serde_json::Value =
            serde_json::from_str(&logs[2].strip_prefix("EVENT_JSON:").unwrap()).unwrap();
        assert_eq!(log2.as_object().unwrap().len(), 3);
        assert_eq!(log2.get("standard").unwrap(), "test_standard");
        assert_eq!(log2.get("version").unwrap(), "3.0.0");
        assert_eq!(log2.get("event").unwrap(), "empty_event");
        assert!(log2.get("data").is_none());
    }
    {
        let log3: serde_json::Value =
            serde_json::from_str(&logs[3].strip_prefix("EVENT_JSON:").unwrap()).unwrap();
        assert_eq!(log3.as_object().unwrap().len(), 4);
        assert_eq!(log3.get("standard").unwrap(), "test_standard");
        assert_eq!(log3.get("version").unwrap(), "4.0.0");
        assert_eq!(log3.get("event").unwrap(), "lifetime_test_a");
        assert_eq!(log3.get("data").unwrap(), "lifetime");
    }
    {
        let log4: serde_json::Value =
            serde_json::from_str(&logs[4].strip_prefix("EVENT_JSON:").unwrap()).unwrap();
        assert_eq!(log4.as_object().unwrap().len(), 4);
        assert_eq!(log4.get("standard").unwrap(), "test_standard");
        assert_eq!(log4.get("version").unwrap(), "5.0.0");
        assert_eq!(log4.get("event").unwrap(), "lifetime_test_b");
        assert_eq!(log4.get("data").unwrap(), "lifetime_b");
    }
    {
        let log5: serde_json::Value =
            serde_json::from_str(&logs[5].strip_prefix("EVENT_JSON:").unwrap()).unwrap();
        assert_eq!(log5.as_object().unwrap().len(), 3);
        assert_eq!(log5.get("standard").unwrap(), "another_standard");
        assert_eq!(log5.get("version").unwrap(), "1.0.0");
        assert_eq!(log5.get("event").unwrap(), "test");
        assert!(log5.get("data").is_none());
    }
}
