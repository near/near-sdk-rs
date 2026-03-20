use std::num::NonZeroU128;

use near_token::NearToken;

/// Allow an access key to spend either an unlimited or limited amount of gas
// This wrapper prevents incorrect construction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Allowance {
    Unlimited,
    Limited(NonZeroU128),
}

impl Allowance {
    pub fn unlimited() -> Allowance {
        Allowance::Unlimited
    }

    /// This will return an None if you try to pass a zero value balance
    pub fn limited(balance: NearToken) -> Option<Allowance> {
        NonZeroU128::new(balance.as_yoctonear()).map(Allowance::Limited)
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod test {
    use crate::{allowance::Allowance, types::NearToken};
    #[test]
    fn test_allowance_debug() {
        let unlimited = Allowance::Unlimited;
        assert_eq!(format!("{:?}", unlimited), "Unlimited");

        let limited = Allowance::Limited(100.try_into().unwrap());
        assert_eq!(format!("{:?}", limited), "Limited(100)");
    }

    #[test]
    fn test_allowance_eq() {
        assert_eq!(Allowance::Unlimited, Allowance::Unlimited);
        assert_eq!(
            Allowance::Limited(100.try_into().unwrap()),
            Allowance::Limited(100.try_into().unwrap())
        );
        assert_ne!(Allowance::Unlimited, Allowance::Limited(100.try_into().unwrap()));
        assert_ne!(
            Allowance::Limited(100.try_into().unwrap()),
            Allowance::Limited(200.try_into().unwrap())
        );
    }

    #[test]
    fn test_allowance_copy() {
        let a = Allowance::Unlimited;
        let b = a; // Copy
        assert_eq!(a, b);

        let c = Allowance::Limited(500.try_into().unwrap());
        let d = c; // Copy
        assert_eq!(c, d);
    }

    #[test]
    fn test_allowance_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(Allowance::Unlimited);
        set.insert(Allowance::Limited(100.try_into().unwrap()));
        set.insert(Allowance::Unlimited); // duplicate
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_allowance_limited_zero_returns_none() {
        assert!(Allowance::limited(NearToken::from_yoctonear(0)).is_none());
    }

    #[test]
    fn test_allowance_limited_nonzero() {
        let allowance = Allowance::limited(NearToken::from_yoctonear(100));
        assert_eq!(allowance, Some(Allowance::Limited(100.try_into().unwrap())));
    }
}
