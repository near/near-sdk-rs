#[macro_export]
macro_rules! map {
    // trailing comma case
    ($($key:expr => $value:expr,)+) => (map!($($key => $value),+));

    ( $($key:expr => $value:expr),* ) => {
        {
            let mut _map = ::std::collections::BTreeMap::new();
            $(
                let _ = _map.insert($key, $value);
            )*
            _map
        }
    };
}
