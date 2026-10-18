//! Soundness harnesses for storage v2 (Oct 17 soundness round).
//!
//! Every harness is one `bolero::check!`: it runs under the random engine in `cargo test`, and
//! the ones marked `kani::proof` also run under Kani with
//! `NEAR_SANDBOX_BIN_PATH=/usr/bin/true cargo kani --tests --lib -p near-sdk --features unstable --harness <name>`.
//! The bound of each claim is stated on the harness. See PROTOTYPE_NOTES.md, "Soundness round".

#[cfg(test)]
pub mod fixture;
#[cfg(test)]
pub mod diff;
#[cfg(test)]
pub mod g1_addressable;
#[cfg(test)]
pub mod cost;
#[cfg(test)]
pub mod g7_stated_cost;
