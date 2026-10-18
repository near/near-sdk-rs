//! Soundness harnesses for storage v2 (Oct 17 soundness round).
//!
//! Every harness is one `bolero::check!`: it runs under the random engine in `cargo test`,
//! and the ones marked `kani::proof` also run under Kani with
//! `cargo kani --tests -p near-sdk --features unstable --harness <name>`.

#[test]
#[cfg_attr(kani, kani::proof)]
#[cfg_attr(kani, kani::unwind(3))]
fn mechanics_probe() {
    bolero::check!().with_type::<(u8, u8)>().for_each(|(a, b)| {
        assert_eq!(a.wrapping_add(*b), b.wrapping_add(*a));
    });
}
