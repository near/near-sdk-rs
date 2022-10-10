pub trait StandardEvent {
    fn format(&self) -> String;
    fn emit(&self);
}

pub struct Event {}
impl Event {
    pub fn emit(standard_event: impl StandardEvent) {
        crate::env::log_str(&format!("EVENT_JSON:{}", &standard_event.format()));
    }
}

#[cfg(test)]
pub mod tests {

    use crate::test_utils::get_logs;
    use crate::{near_bindgen, AccountId};

    use super::Event;
    use super::StandardEvent;
    use crate as near_sdk;

    #[near_bindgen(events)]
    pub enum TestEvents<'a, 'b, T>
    where
        T: crate::serde::Serialize + Clone,
    {
        #[event_meta(standard = "swap_standard", version = "1.0.0")]
        Swap {
            token_in: AccountId,
            token_out: AccountId,
            amount_in: u128,
            amount_out: u128,
            test: T,
        },

        #[event_meta(standard = "string_standard", version = "2.0.0")]
        StringEvent(String),

        #[event_meta(standard = "empty_standard", version = "3.0.0")]
        EmptyEvent,

        #[event_meta(standard = "lifetime_std", version = "4.0.0")]
        LifetimeTestA(&'a str),

        #[event_meta(standard = "lifetime_std", version = "4.0.0")]
        LifetimeTestB(&'b str),
    }

    #[test]
    fn test_json_emit() {
        let token_in: AccountId = "wrap.near".parse().unwrap();
        let token_out: AccountId = "test.near".parse().unwrap();
        let amount_in: u128 = 100;
        let amount_out: u128 = 200;
        Event::emit(TestEvents::Swap {
            token_in,
            token_out,
            amount_in,
            amount_out,
            test: String::from("tst"),
        });

        Event::emit(TestEvents::StringEvent::<String>(String::from("string")));

        Event::emit(TestEvents::EmptyEvent::<String>);

        Event::emit(TestEvents::LifetimeTestA::<String>("lifetime"));

        TestEvents::LifetimeTestB::<String>("lifetime_b").emit();

        let logs = get_logs();

        assert!(
            logs[0]
                == r#"EVENT_JSON:{"standard":"swap_standard","version":"1.0.0","event":"swap","data":{"token_in":"wrap.near","token_out":"test.near","amount_in":100,"amount_out":200,"test":"tst"}}"#
        );
        assert!(
            logs[1]
                == r#"EVENT_JSON:{"standard":"string_standard","version":"2.0.0","event":"string_event","data":"string"}"#
        );
        assert!(
            logs[2]
                == r#"EVENT_JSON:{"standard":"empty_standard","version":"3.0.0","event":"empty_event"}"#
        );
        assert!(
            logs[3]
                == r#"EVENT_JSON:{"standard":"lifetime_std","version":"4.0.0","event":"lifetime_test_a","data":"lifetime"}"#
        );
        assert!(
            logs[4]
                == r#"EVENT_JSON:{"standard":"lifetime_std","version":"4.0.0","event":"lifetime_test_b","data":"lifetime_b"}"#
        );
    }
}
