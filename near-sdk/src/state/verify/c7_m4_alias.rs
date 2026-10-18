//! Bug class C7 / decision M4, root-byte and empty-prefix aliasing. A v1 map writes keys
//! `prefix ++ borsh(key)` (Identity hasher) that are caller-shaped; every v2 key starts with
//! the root byte. `check_v1_prefix` (run when a `Migrating` field is placed or loaded) admits a
//! prefix only if it is non-empty and does not start with the root byte. These harnesses prove
//! the admitted prefixes are disjoint from every v2 key, and that each rejection is for one of
//! the two stated reasons. The wiring and the unguarded exception (a bare v1 field at the root)
//! are pinned through the real derive in `tests/state_v2_verify.rs`.

use super::g1_addressable::SPath;
use crate::state::key::{ENTRY_HASH_LEN, Path};
use crate::state::{V1PrefixError, check_v1_prefix};

/// Bound (Kani): any root byte, any v1 prefix of 0..=3 bytes, a v1 key suffix that is either
/// any 0..=4 bytes or (forged) the v2 key's own tail after the prefix length, so an alias is
/// reachable whenever the prefix matches the v2 key's start; any v2 map-entry key `root . field(t) . kind(k) . entry(h, enc)` with any nonzero `u32` tag,
/// kind, hash and an encoded key of 0..=2 bytes. If the guard admits the prefix, the v1 key
/// differs from the v2 key; if it rejects, the reason is the empty prefix or the root byte.
/// Random engine: 100,000 cases.
#[test]
#[cfg_attr(kani, kani::proof)]
#[cfg_attr(kani, kani::unwind(20))]
fn c7_admitted_v1_prefixes_never_alias_v2_keys() {
    let g = (
        bolero::produce::<u8>(),
        bolero::produce::<([u8; 3], u8)>(),
        bolero::produce::<([u8; 4], u8, bool)>(),
        bolero::produce::<(u32, u8, [u8; ENTRY_HASH_LEN], [u8; 2], u8)>(),
    );
    bolero::check!().with_generator(g).with_iterations(100_000).for_each(|(root, (p, pl), (s, sl, forge), (t, k, h, e, el))| {
        let prefix = &p[..(*pl % 4) as usize];
        let suffix = &s[..(*sl % 5) as usize];
        let v2 = Path::root(*root).field((*t).max(1)).kind(*k).entry_hashed(h, &e[..(*el % 3) as usize]);
        let v2 = v2.as_bytes();
        let mut v1 = prefix.to_vec();
        v1.extend_from_slice(if *forge { &v2[prefix.len().min(v2.len())..] } else { suffix });
        match check_v1_prefix(prefix, *root) {
            Ok(()) => assert!(v1 != v2, "an admitted v1 prefix aliased a v2 key"),
            Err(V1PrefixError::Empty) => assert!(prefix.is_empty()),
            Err(V1PrefixError::RootByte(r)) => assert!(r == *root && prefix[0] == *root),
        }
    });
}

/// The same property against PRODUCTION v1 keys (`store::key::{Identity, Sha256, Keccak256}`
/// `to_key` on borsh `String` / `u32` keys) and production v2 keys of the G1 path generator
/// (up to 4 hops, real placement hash). Identity keys under an admitted prefix never equal a
/// v2 key; and an empty prefix lets a caller-chosen `[u8; N]` key equal ANY v2 key (the
/// rejected case is a real alias, not just conservatism). Random engine: 20,000 cases.
#[test]
fn c7_production_v1_keys_vs_production_v2_keys() {
    use crate::store::key::{Identity, Keccak256, Sha256, ToKey};
    let g = (bolero::produce::<SPath>(), bolero::produce::<std::vec::Vec<u8>>().with().len(0usize..=3), bolero::produce::<(std::string::String, u32)>());
    bolero::check!().with_generator(g).with_iterations(20_000).for_each(|(sp, prefix, (ks, ku))| {
        if !sp.wf() {
            return;
        }
        let v2 = sp.encode(true);
        let root = v2[0];
        let ids = [Identity::to_key(prefix, ks, &mut vec![]), Identity::to_key(prefix, ku, &mut vec![])];
        if check_v1_prefix(prefix, root).is_ok() {
            assert!(ids.iter().all(|k| k[..] != v2[..]), "admitted Identity v1 key aliased a v2 key");
        }
        // hashed v1 keys are 32 bytes; a v2 key of another length can never equal them
        for hk in [Sha256::to_key(prefix, ks, &mut vec![]), Keccak256::to_key(prefix, ks, &mut vec![])] {
            if v2.len() != 32 {
                assert_ne!(&hk[..], &v2[..]);
            }
        }
        // the alias the guard exists for: empty prefix, fixed-size byte key = the v2 key
        if v2.len() <= 64 {
            let mut arr = [0u8; 64];
            arr[..v2.len()].copy_from_slice(&v2);
            let forged = Identity::to_key(&[], &arr[..v2.len()].to_vec(), &mut vec![]);
            // borsh(Vec<u8>) adds a 4-byte length, but a borsh fixed array does not:
            let raw: std::vec::Vec<u8> = Identity::to_key(&[], &RawBytes(&v2), &mut vec![]);
            assert_eq!(raw, v2, "an empty-prefix v1 key equals the v2 key");
            assert_ne!(forged, v2);
            assert_eq!(check_v1_prefix(&[], root), Err(V1PrefixError::Empty));
        }
    });
}

/// Borsh of a fixed-size byte array: the bytes themselves (stands in for `[u8; N]` of any N).
struct RawBytes<'a>(&'a [u8]);
impl borsh::BorshSerialize for RawBytes<'_> {
    fn serialize<W: borsh::io::Write>(&self, w: &mut W) -> borsh::io::Result<()> {
        w.write_all(self.0)
    }
}
