pub trait EventJson {
    type EventString: std::convert::AsRef<str>;
    fn format(&self) -> Self::EventString;
    fn emit(&self) {
        crate::env::log_str(&format!("EVENT_JSON:{}", self.format().as_ref()));
    }
}

#[cfg(test)]
pub mod tests {
    use crate::test_utils::get_logs;
    use crate::{near_bindgen, AccountId};

    use super::EventJson;
    use crate as near_sdk;

    #[near_bindgen(event_json(standard = "test_standard", random = "random"), other_random)]
    pub enum TestEvents<'a, 'b, T>
    where
        T: crate::serde::Serialize,
    {
        #[event_version("1.0.0")]
        Swap {
            token_in: AccountId,
            token_out: AccountId,
            amount_in: u128,
            amount_out: u128,
            test: T,
        },

        #[event_version("2.0.0")]
        StringEvent(String),

        #[event_version("3.0.0")]
        EmptyEvent,

        #[event_version("4.0.0")]
        LifetimeTestA(&'a str),

        #[event_version("5.0.0")]
        LifetimeTestB(&'b str),
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

        let logs = get_logs();

        assert!(
            logs[0]
                == r#"EVENT_JSON:{"standard":"test_standard","version":"1.0.0","event":"swap","data":{"token_in":"wrap.near","token_out":"test.near","amount_in":100,"amount_out":200,"test":"tst"}}"#
        );
        assert!(
            logs[1]
                == r#"EVENT_JSON:{"standard":"test_standard","version":"2.0.0","event":"string_event","data":"string"}"#
        );
        assert!(
            logs[2]
                == r#"EVENT_JSON:{"standard":"test_standard","version":"3.0.0","event":"empty_event"}"#
        );
        assert!(
            logs[3]
                == r#"EVENT_JSON:{"standard":"test_standard","version":"4.0.0","event":"lifetime_test_a","data":"lifetime"}"#
        );
        assert!(
            logs[4]
                == r#"EVENT_JSON:{"standard":"test_standard","version":"5.0.0","event":"lifetime_test_b","data":"lifetime_b"}"#
        );
    }
}
