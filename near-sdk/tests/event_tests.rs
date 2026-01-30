use near_sdk::test_utils::get_logs;
use near_sdk::{AccountId, AsNep297Event, near};

#[near(event_json(standard = "test_standard"))]
#[derive(Debug)]
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
    let log0_event = log0_struct.to_nep297_event();
    log0_struct.emit();

    let log1_struct = TestEvents::StringEvent::<String>(String::from("string"));
    let log1_json_expected = log1_struct.to_json();
    let log1_event = log1_struct.to_nep297_event();
    log1_struct.emit();

    let log2_struct = TestEvents::EmptyEvent::<String>;
    let log2_json_expected = log2_struct.to_json();
    let log2_event = log2_struct.to_nep297_event();
    log2_struct.emit();

    let log3_struct = TestEvents::LifetimeTestA::<String>("lifetime");
    let log3_json_expected = log3_struct.to_json();
    let log3_event = log3_struct.to_nep297_event();
    log3_struct.emit();

    let log4_struct = TestEvents::LifetimeTestB::<String>("lifetime_b");
    let log4_json_expected = log4_struct.to_json();
    let log4_event = log4_struct.to_nep297_event();
    log4_struct.emit();

    let log5_struct = private::AnotherEvent::Test;
    let log5_json_expected = log5_struct.to_json();
    let log5_event = log5_struct.to_nep297_event();
    log5_struct.emit();

    let logs = get_logs();

    {
        let log0_str = logs[0].strip_prefix("EVENT_JSON:").unwrap();

        assert_eq!(log0_event.to_event_log(), logs[0]);
        assert_eq!(log0_event.to_json_string(), log0_str);

        let log0: serde_json::Value = serde_json::from_str(log0_str).unwrap();

        assert_eq!(
            log0_str,
            r#"{"standard":"test_standard","version":"1.0.0","event":"swap","data":{"token_in":"wrap.near","token_out":"test.near","amount_in":100,"amount_out":200,"test":"tst"}}"#
        );

        assert_eq!(log0_json_expected, log0);
        assert_eq!(log0_json_expected, log0_event.to_json());

        assert_eq!(log0.as_object().unwrap().len(), 4);
        assert_eq!(log0.get("standard").unwrap(), "test_standard");
        assert_eq!(log0.get("version").unwrap(), "1.0.0");
        assert_eq!(log0.get("event").unwrap(), "swap");

        assert_eq!(log0_event.standard(), "test_standard");
        assert_eq!(log0_event.version(), "1.0.0");
        assert_eq!(log0_event.event(), "swap");

        let data0 = log0.get("data").unwrap();
        assert_eq!(&log0_event.data().unwrap(), data0);
        assert_eq!(data0.as_object().unwrap().len(), 5);
        assert_eq!(data0.get("token_in").unwrap(), "wrap.near");
        assert_eq!(data0.get("token_out").unwrap(), "test.near");
        assert_eq!(data0.get("amount_in").unwrap(), 100);
        assert_eq!(data0.get("amount_out").unwrap(), 200);
        assert_eq!(data0.get("test").unwrap(), "tst");
    }
    {
        let log1_str = logs[1].strip_prefix("EVENT_JSON:").unwrap();

        assert_eq!(log1_event.to_event_log(), logs[1]);
        assert_eq!(log1_event.to_json_string(), log1_str);

        let log1: serde_json::Value = serde_json::from_str(log1_str).unwrap();

        assert_eq!(
            log1_str,
            r#"{"standard":"test_standard","version":"2.0.0","event":"string_event","data":"string"}"#
        );

        assert_eq!(log1_json_expected, log1);
        assert_eq!(log1_json_expected, log1_event.to_json());

        assert_eq!(log1.as_object().unwrap().len(), 4);
        assert_eq!(log1.get("standard").unwrap(), "test_standard");
        assert_eq!(log1.get("version").unwrap(), "2.0.0");
        assert_eq!(log1.get("event").unwrap(), "string_event");
        assert_eq!(log1.get("data").unwrap(), "string");

        assert_eq!(log1_event.standard(), "test_standard");
        assert_eq!(log1_event.version(), "2.0.0");
        assert_eq!(log1_event.event(), "string_event");
        assert_eq!(log1_event.data(), Some(log1.get("data").unwrap().clone()));
    }
    {
        let log2_str = logs[2].strip_prefix("EVENT_JSON:").unwrap();

        assert_eq!(log2_event.to_event_log(), logs[2]);
        assert_eq!(log2_event.to_json_string(), log2_str);

        let log2: serde_json::Value = serde_json::from_str(log2_str).unwrap();

        assert_eq!(
            log2_str,
            r#"{"standard":"test_standard","version":"3.0.0","event":"empty_event"}"#
        );

        assert_eq!(log2_json_expected, log2);
        assert_eq!(log2_json_expected, log2_event.to_json());

        assert_eq!(log2.as_object().unwrap().len(), 3);
        assert_eq!(log2.get("standard").unwrap(), "test_standard");
        assert_eq!(log2.get("version").unwrap(), "3.0.0");
        assert_eq!(log2.get("event").unwrap(), "empty_event");
        assert!(log2.get("data").is_none());

        assert_eq!(log2_event.standard(), "test_standard");
        assert_eq!(log2_event.version(), "3.0.0");
        assert_eq!(log2_event.event(), "empty_event");
        assert!(log2_event.data().is_none());
    }
    {
        let log3_str = logs[3].strip_prefix("EVENT_JSON:").unwrap();

        assert_eq!(log3_event.to_event_log(), logs[3]);
        assert_eq!(log3_event.to_json_string(), log3_str);

        let log3: serde_json::Value = serde_json::from_str(log3_str).unwrap();

        assert_eq!(
            log3_str,
            r#"{"standard":"test_standard","version":"4.0.0","event":"lifetime_test_a","data":"lifetime"}"#
        );

        assert_eq!(log3_json_expected, log3);
        assert_eq!(log3_json_expected, log3_event.to_json());

        assert_eq!(log3.as_object().unwrap().len(), 4);
        assert_eq!(log3.get("standard").unwrap(), "test_standard");
        assert_eq!(log3.get("version").unwrap(), "4.0.0");
        assert_eq!(log3.get("event").unwrap(), "lifetime_test_a");
        assert_eq!(log3.get("data").unwrap(), "lifetime");

        assert_eq!(log3_event.standard(), "test_standard");
        assert_eq!(log3_event.version(), "4.0.0");
        assert_eq!(log3_event.event(), "lifetime_test_a");
        assert_eq!(log3_event.data(), Some(log3.get("data").unwrap().clone()));
    }
    {
        let log4_str = logs[4].strip_prefix("EVENT_JSON:").unwrap();

        assert_eq!(log4_event.to_event_log(), logs[4]);
        assert_eq!(log4_event.to_json_string(), log4_str);

        let log4: serde_json::Value = serde_json::from_str(log4_str).unwrap();

        assert_eq!(
            log4_str,
            r#"{"standard":"test_standard","version":"5.0.0","event":"lifetime_test_b","data":"lifetime_b"}"#
        );

        assert_eq!(log4_json_expected, log4);
        assert_eq!(log4_json_expected, log4_event.to_json());

        assert_eq!(log4.as_object().unwrap().len(), 4);
        assert_eq!(log4.get("standard").unwrap(), "test_standard");
        assert_eq!(log4.get("version").unwrap(), "5.0.0");
        assert_eq!(log4.get("event").unwrap(), "lifetime_test_b");
        assert_eq!(log4.get("data").unwrap(), "lifetime_b");

        assert_eq!(log4_event.standard(), "test_standard");
        assert_eq!(log4_event.version(), "5.0.0");
        assert_eq!(log4_event.event(), "lifetime_test_b");
        assert_eq!(log4_event.data(), Some(log4.get("data").unwrap().clone()));
    }
    {
        let log5_str = logs[5].strip_prefix("EVENT_JSON:").unwrap();

        assert_eq!(log5_event.to_event_log(), logs[5]);
        assert_eq!(log5_event.to_json_string(), log5_str);

        let log5: serde_json::Value = serde_json::from_str(log5_str).unwrap();

        assert_eq!(log5_str, r#"{"standard":"another_standard","version":"1.0.0","event":"test"}"#);

        assert_eq!(log5_json_expected, log5);
        assert_eq!(log5_json_expected, log5_event.to_json());

        assert_eq!(log5.as_object().unwrap().len(), 3);
        assert_eq!(log5.get("standard").unwrap(), "another_standard");
        assert_eq!(log5.get("version").unwrap(), "1.0.0");
        assert_eq!(log5.get("event").unwrap(), "test");
        assert!(log5.get("data").is_none());

        assert_eq!(log5_event.standard(), "another_standard");
        assert_eq!(log5_event.version(), "1.0.0");
        assert_eq!(log5_event.event(), "test");
        assert!(log5_event.data().is_none());
    }
}

// Gas consumption tests for emit() similar to store_performance_tests.rs
#[cfg(target_os = "linux")]
mod emit_gas_consumption_tests {
    use super::*;
    use near_sdk::env;
    use near_sdk::test_utils::test_env::{alice, bob};

    fn setup_test_env() {
        let context = near_sdk::test_utils::VMContextBuilder::new()
            .current_account_id(alice())
            .predecessor_account_id(bob())
            .build();
        near_sdk::testing_env!(context);
    }

    #[track_caller]
    fn perform_gas_asserts(gas_used: u64, event_type: &str, expected_max: u64) {
        let caller = std::panic::Location::caller();
        println!(
            "{}: Consumed {} out of {}",
            event_type,
            near_gas::NearGas::from_gas(gas_used),
            near_gas::NearGas::from_gas(expected_max)
        );
        assert!(
            gas_used <= expected_max,
            "Gas consumption too high for {} at {}:{}:{}: {} (expected at most {})",
            event_type,
            caller.file(),
            caller.line(),
            caller.column(),
            gas_used,
            expected_max
        );
    }

    #[test]
    fn test_emit_performance() {
        setup_test_env();

        // Test simple Swap event emission
        {
            let token_in: AccountId = "wrap.near".parse().unwrap();
            let token_out: AccountId = "test.near".parse().unwrap();
            let event = TestEvents::Swap {
                token_in,
                token_out,
                amount_in: 100,
                amount_out: 200,
                test: String::from("test"),
            };

            let gas_before = env::used_gas();
            event.emit();
            let gas_after = env::used_gas();
            let gas_used = gas_after.as_gas() - gas_before.as_gas();

            perform_gas_asserts(gas_used, "Swap event emit", 63796097058 + 3000000000);
        }

        // Test empty event emission (minimal overhead)
        {
            let event = TestEvents::EmptyEvent::<String>;
            let gas_before = env::used_gas();
            event.emit();
            let gas_after = env::used_gas();
            let gas_used = gas_after.as_gas() - gas_before.as_gas();

            perform_gas_asserts(gas_used, "Empty event emit", 34172359170 + 1700000000);
        }

        // Test string event emission
        {
            let event = TestEvents::StringEvent::<String>(String::from("test_string"));
            let gas_before = env::used_gas();
            event.emit();
            let gas_after = env::used_gas();
            let gas_used = gas_after.as_gas() - gas_before.as_gas();

            perform_gas_asserts(gas_used, "String event emit", 40961132436 + 2000000000);
        }
    }

    #[test]
    fn test_emit_with_varying_data_sizes() {
        setup_test_env();

        // Small data (< 100 bytes)
        {
            let event = TestEvents::StringEvent::<String>(String::from("small"));
            let gas_before = env::used_gas();
            event.emit();
            let gas_after = env::used_gas();
            let gas_used = gas_after.as_gas() - gas_before.as_gas();

            println!("{}", near_gas::NearGas::from_tgas(3).as_gas());
            perform_gas_asserts(gas_used, "Small data event", 39109648818 + 2000000000);
        }

        // Medium data (~1KB)
        {
            let medium_string = "x".repeat(1024);
            let event = TestEvents::StringEvent::<String>(medium_string);

            let gas_before = env::used_gas();
            event.emit();
            let gas_after = env::used_gas();
            let gas_used = gas_after.as_gas() - gas_before.as_gas();

            perform_gas_asserts(gas_used, "Medium data event (1KB)", 353553283275 + 15000000000);
        }

        // Large data (~10KB)
        {
            let large_string = "x".repeat(10000);
            let event = TestEvents::StringEvent::<String>(large_string);

            let gas_before = env::used_gas();
            event.emit();
            let gas_after = env::used_gas();
            let gas_used = gas_after.as_gas() - gas_before.as_gas();

            perform_gas_asserts(gas_used, "Large data event (10KB)", 3123372775803 + 150000000000);
        }
    }

    #[test]
    fn test_emit_multiple_events_performance() {
        setup_test_env();

        let iterations = 50;
        let token_in: AccountId = "wrap.near".parse().unwrap();
        let token_out: AccountId = "test.near".parse().unwrap();

        let gas_before = env::used_gas();

        for i in 0..iterations {
            let event = TestEvents::Swap {
                token_in: token_in.clone(),
                token_out: token_out.clone(),
                amount_in: i as u128,
                amount_out: (i * 2) as u128,
                test: format!("test_{}", i),
            };
            event.emit();
        }

        let gas_after = env::used_gas();
        let total_gas_used = gas_after.as_gas() - gas_before.as_gas();

        perform_gas_asserts(total_gas_used, "Total gas used", 3184545730536 + 150000000000);
    }
}
