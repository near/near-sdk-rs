use near_sdk::test_utils::get_logs;
use near_sdk::{near, AccountId, AsNep297Event};

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

    let log0_struct =
        TestEvents::Swap { token_in, token_out, amount_in, amount_out, test: String::from("tst") };
    let log0_json_expected = log0_struct.to_json();
    let log0_nep297_event = log0_struct.to_nep297_event();
    let log0_standard = log0_struct.standard();
    let log0_version = log0_struct.version();
    let log0_event = log0_struct.event();
    let log0_data = log0_struct.data();
    log0_struct.emit();

    let log1_struct = TestEvents::StringEvent::<String>(String::from("string"));
    let log1_json_expected = log1_struct.to_json();
    let log1_nep297_event = log1_struct.to_nep297_event();
    let log1_standard = log1_struct.standard();
    let log1_version = log1_struct.version();
    let log1_event = log1_struct.event();
    let log1_data = log1_struct.data();
    log1_struct.emit();

    let log2_struct = TestEvents::EmptyEvent::<String>;
    let log2_json_expected = log2_struct.to_json();
    let log2_nep297_event = log2_struct.to_nep297_event();
    let log2_standard = log2_struct.standard();
    let log2_version = log2_struct.version();
    let log2_event = log2_struct.event();
    let log2_data = log2_struct.data();
    log2_struct.emit();

    let log3_struct = TestEvents::LifetimeTestA::<String>("lifetime");
    let log3_json_expected = log3_struct.to_json();
    let log3_nep297_event = log3_struct.to_nep297_event();
    let log3_standard = log3_struct.standard();
    let log3_version = log3_struct.version();
    let log3_event = log3_struct.event();
    let log3_data = log3_struct.data();
    log3_struct.emit();

    let log4_struct = TestEvents::LifetimeTestB::<String>("lifetime_b");
    let log4_json_expected = log4_struct.to_json();
    let log4_nep297_event = log4_struct.to_nep297_event();
    let log4_standard = log4_struct.standard();
    let log4_version = log4_struct.version();
    let log4_event = log4_struct.event();
    let log4_data = log4_struct.data();
    log4_struct.emit();

    let log5_struct = private::AnotherEvent::Test;
    let log5_json_expected = log5_struct.to_json();
    let log5_nep297_event = log5_struct.to_nep297_event();
    let log5_standard = log5_struct.standard();
    let log5_version = log5_struct.version();
    let log5_event = log5_struct.event();
    let log5_data = log5_struct.data();
    log5_struct.emit();

    let logs = get_logs();

    {
        let log0_str = logs[0].strip_prefix("EVENT_JSON:").unwrap();

        let log0: serde_json::Value = serde_json::from_str(log0_str).unwrap();

        assert_eq!(
            log0_str,
            r#"{"standard":"test_standard","version":"1.0.0","event":"swap","data":{"token_in":"wrap.near","token_out":"test.near","amount_in":100,"amount_out":200,"test":"tst"}}"#
        );

        assert_eq!(log0_json_expected, log0);

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

        assert_eq!(log0_event, "swap");
        assert_eq!(log0_version, "1.0.0");
        assert_eq!(log0_standard, "test_standard");
        assert_eq!(&log0_data.unwrap(), data0);
        assert_eq!(log0_nep297_event.to_json_string(), log0_str);
        assert_eq!(log0_nep297_event.to_event_log(), logs[0]);
        assert_eq!(log0_nep297_event.standard, "test_standard");
        assert_eq!(log0_nep297_event.version, "1.0.0");
        assert_eq!(log0_nep297_event.event, "swap");
        assert_eq!(&log0_nep297_event.data.unwrap(), data0);
    }
    {
        let log1_str = logs[1].strip_prefix("EVENT_JSON:").unwrap();

        let log1: serde_json::Value = serde_json::from_str(log1_str).unwrap();

        assert_eq!(
            log1_str,
            r#"{"standard":"test_standard","version":"2.0.0","event":"string_event","data":"string"}"#
        );

        assert_eq!(log1_json_expected, log1);

        assert_eq!(log1.as_object().unwrap().len(), 4);
        assert_eq!(log1.get("standard").unwrap(), "test_standard");
        assert_eq!(log1.get("version").unwrap(), "2.0.0");
        assert_eq!(log1.get("event").unwrap(), "string_event");
        assert_eq!(log1.get("data").unwrap(), "string");

        assert_eq!(log1_event, "string_event");
        assert_eq!(log1_version, "2.0.0");
        assert_eq!(log1_standard, "test_standard");
        assert_eq!(&log1_data.unwrap(), log1.get("data").unwrap());
        assert_eq!(log1_nep297_event.to_json_string(), log1_str);
        assert_eq!(log1_nep297_event.to_event_log(), logs[1]);
        assert_eq!(log1_nep297_event.standard, "test_standard");
        assert_eq!(log1_nep297_event.version, "2.0.0");
        assert_eq!(log1_nep297_event.event, "string_event");
        assert_eq!(&log1_nep297_event.data.unwrap(), log1.get("data").unwrap());
    }
    {
        let log2_str = logs[2].strip_prefix("EVENT_JSON:").unwrap();

        let log2: serde_json::Value = serde_json::from_str(log2_str).unwrap();

        assert_eq!(
            log2_str,
            r#"{"standard":"test_standard","version":"3.0.0","event":"empty_event"}"#
        );

        assert_eq!(log2_json_expected, log2);

        assert_eq!(log2.as_object().unwrap().len(), 3);
        assert_eq!(log2.get("standard").unwrap(), "test_standard");
        assert_eq!(log2.get("version").unwrap(), "3.0.0");
        assert_eq!(log2.get("event").unwrap(), "empty_event");
        assert!(log2.get("data").is_none());

        assert_eq!(log2_event, "empty_event");
        assert_eq!(log2_version, "3.0.0");
        assert_eq!(log2_standard, "test_standard");
        assert!(log2_data.is_none());
        assert_eq!(log2_nep297_event.to_json_string(), log2_str);
        assert_eq!(log2_nep297_event.to_event_log(), logs[2]);
        assert_eq!(log2_nep297_event.standard, "test_standard");
        assert_eq!(log2_nep297_event.version, "3.0.0");
        assert_eq!(log2_nep297_event.event, "empty_event");
        assert!(log2_nep297_event.data.is_none());
    }
    {
        let log3_str = logs[3].strip_prefix("EVENT_JSON:").unwrap();

        let log3: serde_json::Value = serde_json::from_str(log3_str).unwrap();

        assert_eq!(
            log3_str,
            r#"{"standard":"test_standard","version":"4.0.0","event":"lifetime_test_a","data":"lifetime"}"#
        );

        assert_eq!(log3_json_expected, log3);

        assert_eq!(log3.as_object().unwrap().len(), 4);
        assert_eq!(log3.get("standard").unwrap(), "test_standard");
        assert_eq!(log3.get("version").unwrap(), "4.0.0");
        assert_eq!(log3.get("event").unwrap(), "lifetime_test_a");
        assert_eq!(log3.get("data").unwrap(), "lifetime");

        assert_eq!(log3_event, "lifetime_test_a");
        assert_eq!(log3_version, "4.0.0");
        assert_eq!(log3_standard, "test_standard");
        assert_eq!(&log3_data.unwrap(), log3.get("data").unwrap());
        assert_eq!(log3_nep297_event.to_json_string(), log3_str);
        assert_eq!(log3_nep297_event.to_event_log(), logs[3]);
        assert_eq!(log3_nep297_event.standard, "test_standard");
        assert_eq!(log3_nep297_event.version, "4.0.0");
        assert_eq!(log3_nep297_event.event, "lifetime_test_a");
        assert_eq!(&log3_nep297_event.data.unwrap(), log3.get("data").unwrap());
    }
    {
        let log4_str = logs[4].strip_prefix("EVENT_JSON:").unwrap();

        let log4: serde_json::Value = serde_json::from_str(log4_str).unwrap();

        assert_eq!(
            log4_str,
            r#"{"standard":"test_standard","version":"5.0.0","event":"lifetime_test_b","data":"lifetime_b"}"#
        );

        assert_eq!(log4_json_expected, log4);

        assert_eq!(log4.as_object().unwrap().len(), 4);
        assert_eq!(log4.get("standard").unwrap(), "test_standard");
        assert_eq!(log4.get("version").unwrap(), "5.0.0");
        assert_eq!(log4.get("event").unwrap(), "lifetime_test_b");
        assert_eq!(log4.get("data").unwrap(), "lifetime_b");

        assert_eq!(log4_event, "lifetime_test_b");
        assert_eq!(log4_version, "5.0.0");
        assert_eq!(log4_standard, "test_standard");
        assert_eq!(&log4_data.unwrap(), log4.get("data").unwrap());
        assert_eq!(log4_nep297_event.to_json_string(), log4_str);
        assert_eq!(log4_nep297_event.to_event_log(), logs[4]);
        assert_eq!(log4_nep297_event.standard, "test_standard");
        assert_eq!(log4_nep297_event.version, "5.0.0");
        assert_eq!(log4_nep297_event.event, "lifetime_test_b");
        assert_eq!(&log4_nep297_event.data.unwrap(), log4.get("data").unwrap());
    }
    {
        let log5_str = logs[5].strip_prefix("EVENT_JSON:").unwrap();

        let log5: serde_json::Value = serde_json::from_str(log5_str).unwrap();

        assert_eq!(log5_str, r#"{"standard":"another_standard","version":"1.0.0","event":"test"}"#);

        assert_eq!(log5_json_expected, log5);

        assert_eq!(log5.as_object().unwrap().len(), 3);
        assert_eq!(log5.get("standard").unwrap(), "another_standard");
        assert_eq!(log5.get("version").unwrap(), "1.0.0");
        assert_eq!(log5.get("event").unwrap(), "test");
        assert!(log5.get("data").is_none());

        assert_eq!(log5_event, "test");
        assert_eq!(log5_version, "1.0.0");
        assert_eq!(log5_standard, "another_standard");
        assert!(log5_data.is_none());
        assert_eq!(log5_nep297_event.to_json_string(), log5_str);
        assert_eq!(log5_nep297_event.to_event_log(), logs[5]);
        assert_eq!(log5_nep297_event.standard, "another_standard");
        assert_eq!(log5_nep297_event.version, "1.0.0");
        assert_eq!(log5_nep297_event.event, "test");
        assert!(log5_nep297_event.data.is_none());
    }
}
