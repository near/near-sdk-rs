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
#[cfg(test)]
pub mod g3_no_debt;
#[cfg(test)]
pub mod g4_one_write;
#[cfg(test)]
pub mod g5_linear_iteration;
#[cfg(test)]
pub mod g2_local;
#[cfg(test)]
pub mod g6_layout;
#[cfg(test)]
pub mod c7_m4_alias;
#[cfg(test)]
pub mod c2_orphans;
#[cfg(test)]
pub mod c5_remove_some;
#[cfg(test)]
pub mod c3_lost_writes;
#[cfg(test)]
pub mod c1_hot_collision;
