mod account;
mod agent;
mod asset;
#[macro_use]
mod macros;
mod mission_control;
mod rate;

#[cfg(all(feature = "wasmcov", target_family = "wasm"))]
wasmcov::near::add_coverage!();
