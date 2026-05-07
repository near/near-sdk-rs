/// Conditionally executes either a NEAR VM host call or pure-Rust fallback.
///
/// * **`host`**: Executes on-chain (`target_arch = "wasm32"`) or when the environment
///   is simulated via the `__near-sdk-unit-testing` feature. Should usually contain
///   calls to [`near-sys`].
/// * **`local`**: Executes off-chain (non-`wasm32` targets). Should usually contain pure-Rust
///   equivalent of the host variant for identical local computation.
macro_rules! execute_target_specific {
    (
        host: $host_block:block,
        local: $local_block:block $(,)?
     ) => {
        #[cfg(any(
            target_arch = "wasm32",
            all(feature = "__near-sdk-unit-testing", not(test), not(doctest))
        ))]
        $host_block

        #[cfg(all(
            not(target_arch = "wasm32"),
            any(not(feature = "__near-sdk-unit-testing"), test, doctest)
        ))]
        $local_block
    };
}
