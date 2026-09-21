//! Strongly-typed wrappers over the [BLS12-381] host functions.
//!
//! These are an ergonomic alternative to the byte-slice `env::bls12381_*` functions, which
//! remain available for callers that already work with packed buffers:
//!
//! * Inputs and outputs are named types ([`G1`], [`G2`], [`Fp`], [`Fp2`], [`Scalar`], …) whose
//!   size and encoding are part of the type, instead of raw `&[u8]` / `Vec<u8>` where the
//!   caller has to hand-pack a flat buffer at exact offsets.
//! * Every fallible operation returns a [`Result`] with an [`Error`] on malformed input.
//! * [`pairing_check`] returns `Result<bool, Error>`, so a malformed input (`Err`) is
//!   distinguishable from a pairing that simply does not hold (`Ok(false)`).
//!
//! # Encoding
//!
//! All encodings match the ones used by the NEAR protocol host functions: the "ZCash"
//! BLS12-381 byte layout with big-endian field elements (the same serialization the zkcrypto
//! `bls12_381` crate produces):
//!
//! | Type             | Size (bytes) | Layout                                          |
//! |------------------|-------------:|-------------------------------------------------|
//! | [`G1`]           | 96           | uncompressed point: big-endian `x` then `y`     |
//! | [`G2`]           | 192          | uncompressed point over `Fp2`                   |
//! | [`G1Compressed`] | 48           | compressed point (flag bits in the top 3 bits)  |
//! | [`G2Compressed`] | 96           | compressed point over `Fp2`                     |
//! | [`Fp`]           | 48           | base-field element, big-endian, must be `< p`   |
//! | [`Fp2`]          | 96           | extension-field element `c1` then `c0`          |
//! | [`Scalar`]       | 32           | 256-bit scalar, **little-endian**               |
//!
//! The additive operations ([`p1_sum`], [`p2_sum`]) take a per-point [`Sign`] so points can be
//! subtracted without materializing their negation.
//!
//! The point at infinity is encoded with the infinity flag set (`0x40` in the first byte),
//! not as all zeros.
//!
//! Note this is **not** the EIP-2537 encoding used by the Ethereum precompiles: that spec
//! zero-pads field elements to 64 bytes (128-byte `G1`, 256-byte `G2`) and defines no
//! compressed form, so EIP-2537 byte vectors are not interchangeable with these types.
//!
//! These functions are batch primitives — folding or mapping every element in a single host
//! call. Prefer passing all your inputs to one call over calling them in a loop.
//!
//! # Subgroup checks
//!
//! A [`G1`] or [`G2`] value can hold any point on its curve, not only points in the prime-order
//! `G1`/`G2` subgroup, and only some functions check subgroup membership:
//!
//! * [`g1_multiexp`], [`g2_multiexp`] and [`pairing_check`] reject points outside the subgroup.
//! * [`p1_sum`], [`p2_sum`], [`p1_decompress`] and [`p2_decompress`] don't check it: per
//!   [NEP-488] they are defined over the whole curve, so neither their inputs nor their outputs
//!   are guaranteed to be in the subgroup.
//! * [`map_fp_to_g1`] and [`map_fp2_to_g2`] always return points in the subgroup.
//!
//! One exception: on protocol versions without the NEP-488 fix (`bls12381_not_in_group_fix`),
//! [`p1_sum`] and [`p1_decompress`] still reject the G1 points `(0, ±2)`, which are on the curve
//! but outside `G1`. So don't rely on these functions to either accept or reject points outside
//! the subgroup.
//!
//! This matters when you combine untrusted points before checking them. For example, if you
//! aggregate public keys with [`p1_sum`] and then run [`pairing_check`] on the result, only the
//! aggregate is checked, not each key. To check every key, aggregate with [`g1_multiexp`]
//! instead, with each scalar set to one: it rejects any key outside `G1` and returns the same
//! sum.
//!
//! # Example: verifying a BLS signature
//!
//! Verification reduces to `e(pubkey, H(m)) == e(g1, signature)`, where `g1` is
//! [`G1::GENERATOR`]. It is rewritten as a single pairing check
//! `e(pubkey, H(m)) * e(-g1, signature) == 1`, with `-g1` being [`G1::NEG_GENERATOR`]:
//!
//! ```no_run
//! use near_sdk::bls12381::{self, Error, G1Compressed, G2Compressed, G1, G2};
//!
//! fn verify_signature(
//!     pubkey_compressed: [u8; 48],    // public key, a compressed G1 point
//!     signature_compressed: [u8; 96], // signature, a compressed G2 point
//!     hashed_message: G2,             // H(m), the message hashed to a G2 point
//! ) -> Result<bool, Error> {
//!     // Decompress the inputs; malformed points return `Err`. Decompression does not check
//!     // subgroup membership, but `pairing_check` below does.
//!     let pubkey = G1Compressed(pubkey_compressed).decompress()?;
//!     let signature = G2Compressed(signature_compressed).decompress()?;
//!
//!     // One batched pairing check over both pairs. `Ok(true)` means the signature is valid,
//!     // `Ok(false)` means it is not, and `Err` means one of the points was malformed.
//!     bls12381::pairing_check(&[(pubkey, hashed_message), (G1::NEG_GENERATOR, signature)])
//! }
//! ```
//!
//! [BLS12-381]: https://datatracker.ietf.org/doc/html/draft-irtf-cfrg-pairing-friendly-curves
//! [NEP-488]: https://github.com/near/NEPs/blob/master/neps/nep-0488.md
use crate::env::{panic_str, read_register};
use crate::environment::env::{ATOMIC_OP_REGISTER, expect_register, read_register_fixed};
use near_sys as sys;

/// Size in bytes of an uncompressed G1 point.
const G1_LEN: usize = 96;
/// Size in bytes of an uncompressed G2 point.
const G2_LEN: usize = 192;
/// Size in bytes of a compressed G1 point.
const G1_COMPRESSED_LEN: usize = 48;
/// Size in bytes of a compressed G2 point.
const G2_COMPRESSED_LEN: usize = 96;
/// Size in bytes of a base-field element `Fp`.
const FP_LEN: usize = 48;
/// Size in bytes of an extension-field element `Fp2`.
const FP2_LEN: usize = 96;
/// Size in bytes of a scalar.
const SCALAR_LEN: usize = 32;

/// Error returned when the host rejects an input as malformed.
///
/// This corresponds to the host functions returning `1`: a point that is not on the curve, a
/// field element that is `>=` the modulus, or an otherwise incorrectly encoded input.
/// [`g1_multiexp`], [`g2_multiexp`] and [`pairing_check`] also return it for a point outside the
/// `G1`/`G2` subgroup. The other functions don't check subgroup membership (see
/// [subgroup checks](crate::bls12381#subgroup-checks)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Error {
    /// One of the inputs was not a valid encoding of the expected curve or field element.
    InvalidInput,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::InvalidInput => f.write_str("invalid BLS12-381 input"),
        }
    }
}

impl std::error::Error for Error {}

/// Sign of a point in the additive operations ([`p1_sum`] / [`p2_sum`]).
///
/// [`Sign::Negative`] adds the negation of the point, i.e. subtracts it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Sign {
    /// Add the point.
    #[default]
    Positive,
    /// Subtract the point (add its negation).
    Negative,
}

impl Sign {
    #[inline]
    fn to_byte(self) -> u8 {
        match self {
            Sign::Positive => 0,
            Sign::Negative => 1,
        }
    }
}

/// Defines a fixed-size byte-array newtype with the common conversions.
macro_rules! byte_newtype {
    ($(#[$meta:meta])* $name:ident, $len:expr) => {
        $(#[$meta])*
        #[repr(transparent)]
        #[derive(Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name(pub [u8; $len]);

        impl $name {
            /// Wraps a fixed-size byte array without validating its contents. The bytes are
            /// only checked when passed to a host function.
            #[inline]
            pub const fn new(bytes: [u8; $len]) -> Self {
                Self(bytes)
            }

            /// Returns a reference to the underlying bytes.
            #[inline]
            pub const fn as_bytes(&self) -> &[u8; $len] {
                &self.0
            }

            /// Consumes the value, returning the underlying bytes.
            #[inline]
            pub const fn into_bytes(self) -> [u8; $len] {
                self.0
            }
        }

        impl From<[u8; $len]> for $name {
            #[inline]
            fn from(bytes: [u8; $len]) -> Self {
                Self(bytes)
            }
        }

        impl AsRef<[u8]> for $name {
            #[inline]
            fn as_ref(&self) -> &[u8] {
                &self.0
            }
        }

        impl TryFrom<&[u8]> for $name {
            type Error = Error;
            #[inline]
            fn try_from(bytes: &[u8]) -> Result<Self, Error> {
                bytes.try_into().map(Self).map_err(|_| Error::InvalidInput)
            }
        }

        impl core::fmt::Debug for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                // The full array is large; print the type name and hex to stay readable.
                write!(f, "{}(", stringify!($name))?;
                for b in &self.0 {
                    write!(f, "{:02x}", b)?;
                }
                f.write_str(")")
            }
        }
    };
}

byte_newtype!(
    /// An uncompressed point on the BLS12-381 curve `E(Fp)` (96 bytes: big-endian `x` then `y`).
    ///
    /// A `G1` is not necessarily in the prime-order `G1` subgroup; see
    /// [subgroup checks](crate::bls12381#subgroup-checks).
    G1,
    G1_LEN
);
byte_newtype!(
    /// An uncompressed point on the twisted BLS12-381 curve `E'(Fp2)` (192 bytes).
    ///
    /// A `G2` is not necessarily in the prime-order `G2` subgroup; see
    /// [subgroup checks](crate::bls12381#subgroup-checks).
    G2,
    G2_LEN
);
byte_newtype!(
    /// A compressed BLS12-381 G1 point (48 bytes, with flag bits in the top 3 bits).
    G1Compressed,
    G1_COMPRESSED_LEN
);
byte_newtype!(
    /// A compressed BLS12-381 G2 point (96 bytes).
    G2Compressed,
    G2_COMPRESSED_LEN
);
byte_newtype!(
    /// A BLS12-381 base-field element `Fp` (48 bytes, big-endian, must be less than the field modulus).
    Fp,
    FP_LEN
);
byte_newtype!(
    /// A BLS12-381 quadratic extension-field element `Fp2` (96 bytes, encoded as `c1` then `c0`).
    Fp2,
    FP2_LEN
);
byte_newtype!(
    /// A 256-bit scalar (32 bytes, **little-endian**).
    Scalar,
    SCALAR_LEN
);

/// Decodes a lowercase hex string into a byte array at compile time.
const fn decode_hex<const N: usize>(hex: &str) -> [u8; N] {
    const fn nibble(c: u8) -> u8 {
        match c {
            b'0'..=b'9' => c - b'0',
            b'a'..=b'f' => c - b'a' + 10,
            _ => panic!("invalid hex digit"),
        }
    }
    let hex = hex.as_bytes();
    assert!(hex.len() == 2 * N, "hex string has the wrong length");
    let mut out = [0u8; N];
    let mut i = 0;
    while i < N {
        out[i] = (nibble(hex[2 * i]) << 4) | nibble(hex[2 * i + 1]);
        i += 1;
    }
    out
}

impl G1 {
    /// The standard generator of the `G1` subgroup.
    pub const GENERATOR: Self = Self(decode_hex(concat!(
        // x
        "17f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb",
        // y
        "08b3f481e3aaa0f1a09e30ed741d8ae4fcf5e095d5d00af600db18cb2c04b3edd03cc744a2888ae40caa232946c5e7e1",
    )));

    /// The negation of [`G1::GENERATOR`], as used in the `e(-g1, signature)` term when checking
    /// a signature whose public key is in `G1`.
    pub const NEG_GENERATOR: Self = Self(decode_hex(concat!(
        // x
        "17f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb",
        // p - y
        "114d1d6855d545a8aa7d76c8cf2e21f267816aef1db507c96655b9d5caac42364e6f38ba0ecb751bad54dcd6b939c2ca",
    )));
}

impl G2 {
    /// The standard generator of the `G2` subgroup.
    pub const GENERATOR: Self = Self(decode_hex(concat!(
        // x.c1
        "13e02b6052719f607dacd3a088274f65596bd0d09920b61ab5da61bbdc7f5049334cf11213945d57e5ac7d055d042b7e",
        // x.c0
        "024aa2b2f08f0a91260805272dc51051c6e47ad4fa403b02b4510b647ae3d1770bac0326a805bbefd48056c8c121bdb8",
        // y.c1
        "0606c4a02ea734cc32acd2b02bc28b99cb3e287e85a763af267492ab572e99ab3f370d275cec1da1aaa9075ff05f79be",
        // y.c0
        "0ce5d527727d6e118cc9cdc6da2e351aadfd9baa8cbdd3a76d429a695160d12c923ac9cc3baca289e193548608b82801",
    )));

    /// The negation of [`G2::GENERATOR`], as used in the `e(signature, -g2)` term when checking
    /// a signature whose public key is in `G2`.
    pub const NEG_GENERATOR: Self = Self(decode_hex(concat!(
        // x.c1
        "13e02b6052719f607dacd3a088274f65596bd0d09920b61ab5da61bbdc7f5049334cf11213945d57e5ac7d055d042b7e",
        // x.c0
        "024aa2b2f08f0a91260805272dc51051c6e47ad4fa403b02b4510b647ae3d1770bac0326a805bbefd48056c8c121bdb8",
        // p - y.c1
        "13fa4d4a0ad8b1ce186ed5061789213d993923066dddaf1040bc3ff59f825c78df74f2d75467e25e0f55f8a00fa030ed",
        // p - y.c0
        "0d1b3cc2c7027888be51d9ef691d77bcb679afda66c73f17f9ee3837a55024f78c71363275a75d75d86bab79f74782aa",
    )));
}

/// Reads a single fixed-size point out of the atomic register after a successful host call.
#[inline]
fn read_point<const N: usize>() -> [u8; N] {
    // SAFETY: only called after a host function returned `0`, guaranteeing the register holds
    // exactly `N` bytes.
    unsafe { read_register_fixed::<N>(ATOMIC_OP_REGISTER) }
}

/// Reads `count` fixed-size points out of the atomic register after a successful host call.
#[inline]
fn read_points<const N: usize, T: From<[u8; N]>>(count: usize) -> Vec<T> {
    // On success the host always writes the register (possibly empty) with exactly one `N`-byte
    // point per input. Anything else means the host broke that contract, so abort instead of
    // returning a truncated batch that callers would pair up with the wrong inputs.
    let raw = expect_register(read_register(ATOMIC_OP_REGISTER));
    if raw.len() != count * N {
        panic_str("BLS12-381 host function returned an unexpected number of points");
    }
    raw.chunks_exact(N)
        .map(|chunk| {
            let arr: [u8; N] = chunk.try_into().expect("chunks_exact yields N-byte chunks");
            T::from(arr)
        })
        .collect()
}

/// Compute the BLS12-381 G1 sum over the given signed points.
///
/// Returns [`Error::InvalidInput`] if any point is not correctly encoded or not on the curve.
///
/// This does **not** check that the points are in the `G1` subgroup, so the inputs and the result
/// may lie outside it. Use [`g1_multiexp`] if you need that check; see
/// [subgroup checks](crate::bls12381#subgroup-checks).
pub fn p1_sum(summands: &[(Sign, G1)]) -> Result<G1, Error> {
    let mut buf = Vec::with_capacity(summands.len() * (1 + G1_LEN));
    for (sign, point) in summands {
        buf.push(sign.to_byte());
        buf.extend_from_slice(&point.0);
    }
    let code =
        unsafe { sys::bls12381_p1_sum(buf.len() as _, buf.as_ptr() as _, ATOMIC_OP_REGISTER) };
    match code {
        0 => Ok(G1(read_point::<G1_LEN>())),
        _ => Err(Error::InvalidInput),
    }
}

/// Compute the BLS12-381 G2 sum over the given signed points.
///
/// Returns [`Error::InvalidInput`] if any point is not correctly encoded or not on the curve.
///
/// This does **not** check that the points are in the `G2` subgroup, so the inputs and the result
/// may lie outside it. Use [`g2_multiexp`] if you need that check; see
/// [subgroup checks](crate::bls12381#subgroup-checks).
pub fn p2_sum(summands: &[(Sign, G2)]) -> Result<G2, Error> {
    let mut buf = Vec::with_capacity(summands.len() * (1 + G2_LEN));
    for (sign, point) in summands {
        buf.push(sign.to_byte());
        buf.extend_from_slice(&point.0);
    }
    let code =
        unsafe { sys::bls12381_p2_sum(buf.len() as _, buf.as_ptr() as _, ATOMIC_OP_REGISTER) };
    match code {
        0 => Ok(G2(read_point::<G2_LEN>())),
        _ => Err(Error::InvalidInput),
    }
}

/// Compute the BLS12-381 G1 multiexponentiation over the given `(point, scalar)` terms.
///
/// Returns [`Error::InvalidInput`] if any point is not in the G1 subgroup or is malformed.
pub fn g1_multiexp(terms: &[(G1, Scalar)]) -> Result<G1, Error> {
    let mut buf = Vec::with_capacity(terms.len() * (G1_LEN + SCALAR_LEN));
    for (point, scalar) in terms {
        buf.extend_from_slice(&point.0);
        buf.extend_from_slice(&scalar.0);
    }
    let code =
        unsafe { sys::bls12381_g1_multiexp(buf.len() as _, buf.as_ptr() as _, ATOMIC_OP_REGISTER) };
    match code {
        0 => Ok(G1(read_point::<G1_LEN>())),
        _ => Err(Error::InvalidInput),
    }
}

/// Compute the BLS12-381 G2 multiexponentiation over the given `(point, scalar)` terms.
///
/// Returns [`Error::InvalidInput`] if any point is not in the G2 subgroup or is malformed.
pub fn g2_multiexp(terms: &[(G2, Scalar)]) -> Result<G2, Error> {
    let mut buf = Vec::with_capacity(terms.len() * (G2_LEN + SCALAR_LEN));
    for (point, scalar) in terms {
        buf.extend_from_slice(&point.0);
        buf.extend_from_slice(&scalar.0);
    }
    let code =
        unsafe { sys::bls12381_g2_multiexp(buf.len() as _, buf.as_ptr() as _, ATOMIC_OP_REGISTER) };
    match code {
        0 => Ok(G2(read_point::<G2_LEN>())),
        _ => Err(Error::InvalidInput),
    }
}

/// Map base-field elements to G1 points (hash-to-curve, one point per input element).
///
/// Returns [`Error::InvalidInput`] if any element is `>=` the field modulus.
pub fn map_fp_to_g1(elements: &[Fp]) -> Result<Vec<G1>, Error> {
    let mut buf = Vec::with_capacity(elements.len() * FP_LEN);
    for element in elements {
        buf.extend_from_slice(&element.0);
    }
    let code = unsafe {
        sys::bls12381_map_fp_to_g1(buf.len() as _, buf.as_ptr() as _, ATOMIC_OP_REGISTER)
    };
    match code {
        0 => Ok(read_points::<G1_LEN, G1>(elements.len())),
        _ => Err(Error::InvalidInput),
    }
}

/// Map extension-field elements to G2 points (hash-to-curve, one point per input element).
///
/// Returns [`Error::InvalidInput`] if any element is malformed.
pub fn map_fp2_to_g2(elements: &[Fp2]) -> Result<Vec<G2>, Error> {
    let mut buf = Vec::with_capacity(elements.len() * FP2_LEN);
    for element in elements {
        buf.extend_from_slice(&element.0);
    }
    let code = unsafe {
        sys::bls12381_map_fp2_to_g2(buf.len() as _, buf.as_ptr() as _, ATOMIC_OP_REGISTER)
    };
    match code {
        0 => Ok(read_points::<G2_LEN, G2>(elements.len())),
        _ => Err(Error::InvalidInput),
    }
}

/// Decompress compressed G1 points into their uncompressed form (one output per input).
///
/// Returns [`Error::InvalidInput`] if any point is off the curve or incorrectly encoded.
///
/// This does **not** check that the points are in the `G1` subgroup, so the outputs may lie
/// outside it. See [subgroup checks](crate::bls12381#subgroup-checks).
pub fn p1_decompress(points: &[G1Compressed]) -> Result<Vec<G1>, Error> {
    let mut buf = Vec::with_capacity(points.len() * G1_COMPRESSED_LEN);
    for point in points {
        buf.extend_from_slice(&point.0);
    }
    let code = unsafe {
        sys::bls12381_p1_decompress(buf.len() as _, buf.as_ptr() as _, ATOMIC_OP_REGISTER)
    };
    match code {
        0 => Ok(read_points::<G1_LEN, G1>(points.len())),
        _ => Err(Error::InvalidInput),
    }
}

/// Decompress compressed G2 points into their uncompressed form (one output per input).
///
/// Returns [`Error::InvalidInput`] if any point is off the curve or incorrectly encoded.
///
/// This does **not** check that the points are in the `G2` subgroup, so the outputs may lie
/// outside it. See [subgroup checks](crate::bls12381#subgroup-checks).
pub fn p2_decompress(points: &[G2Compressed]) -> Result<Vec<G2>, Error> {
    let mut buf = Vec::with_capacity(points.len() * G2_COMPRESSED_LEN);
    for point in points {
        buf.extend_from_slice(&point.0);
    }
    let code = unsafe {
        sys::bls12381_p2_decompress(buf.len() as _, buf.as_ptr() as _, ATOMIC_OP_REGISTER)
    };
    match code {
        0 => Ok(read_points::<G2_LEN, G2>(points.len())),
        _ => Err(Error::InvalidInput),
    }
}

/// Perform a BLS12-381 pairing check on the given `(G1, G2)` pairs.
///
/// Returns:
/// * `Ok(true)` — the product of pairings equals the identity (the check passes);
/// * `Ok(false)` — the product does not equal the identity (the check fails);
/// * `Err(Error::InvalidInput)` — some point is not on its curve, not in its subgroup, or
///   incorrectly encoded.
pub fn pairing_check(pairs: &[(G1, G2)]) -> Result<bool, Error> {
    let mut buf = Vec::with_capacity(pairs.len() * (G1_LEN + G2_LEN));
    for (g1, g2) in pairs {
        buf.extend_from_slice(&g1.0);
        buf.extend_from_slice(&g2.0);
    }
    let code = unsafe { sys::bls12381_pairing_check(buf.len() as _, buf.as_ptr() as _) };
    match code {
        0 => Ok(true),
        2 => Ok(false),
        _ => Err(Error::InvalidInput),
    }
}

impl G1Compressed {
    /// Decompress this single point. Convenience wrapper over [`p1_decompress`], so it does not
    /// check `G1` subgroup membership either.
    #[inline]
    pub fn decompress(&self) -> Result<G1, Error> {
        p1_decompress(core::slice::from_ref(self))
            .map(|mut v| v.pop().expect("one input yields one output"))
    }
}

impl G2Compressed {
    /// Decompress this single point. Convenience wrapper over [`p2_decompress`], so it does not
    /// check `G2` subgroup membership either.
    #[inline]
    pub fn decompress(&self) -> Result<G2, Error> {
        p2_decompress(core::slice::from_ref(self))
            .map(|mut v| v.pop().expect("one input yields one output"))
    }
}

impl Fp {
    /// Map this single element to a G1 point. Convenience wrapper over [`map_fp_to_g1`].
    #[inline]
    pub fn map_to_g1(&self) -> Result<G1, Error> {
        map_fp_to_g1(core::slice::from_ref(self))
            .map(|mut v| v.pop().expect("one input yields one output"))
    }
}

impl Fp2 {
    /// Map this single element to a G2 point. Convenience wrapper over [`map_fp2_to_g2`].
    #[inline]
    pub fn map_to_g2(&self) -> Result<G2, Error> {
        map_fp2_to_g2(core::slice::from_ref(self))
            .map(|mut v| v.pop().expect("one input yields one output"))
    }
}

#[cfg(test)]
mod tests {
    use crate::bls12381::{self, Error, Fp, Fp2, G1, G1Compressed, G2, G2Compressed, Scalar, Sign};

    /// The base-field element `1`, which is `< p` and so a valid input to hash-to-curve.
    fn fp_one() -> Fp {
        let mut bytes = [0u8; 48];
        bytes[47] = 1;
        Fp(bytes)
    }

    /// The extension-field element `1`.
    fn fp2_one() -> Fp2 {
        let mut bytes = [0u8; 96];
        bytes[95] = 1;
        Fp2(bytes)
    }

    /// A little-endian scalar equal to `1`.
    fn scalar_one() -> Scalar {
        let mut bytes = [0u8; 32];
        bytes[0] = 1;
        Scalar(bytes)
    }

    /// The G1 point at infinity. NEAR encodes it with the infinity flag (`0x40`) set in the
    /// first byte, *not* as all zeros.
    fn g1_identity() -> G1 {
        let mut bytes = [0u8; 96];
        bytes[0] = 0x40;
        G1(bytes)
    }

    /// The G2 point at infinity (infinity flag in the first byte).
    fn g2_identity() -> G2 {
        let mut bytes = [0u8; 192];
        bytes[0] = 0x40;
        G2(bytes)
    }

    /// A valid compressed G1 point, reused from the deprecated-API decompress test.
    const VALID_G1_COMPRESSED: [u8; 48] = [
        185, 110, 35, 139, 110, 142, 126, 177, 120, 97, 234, 41, 91, 204, 20, 203, 207, 103, 224,
        112, 176, 18, 102, 59, 68, 107, 137, 231, 10, 71, 183, 63, 198, 228, 242, 206, 195, 124,
        70, 91, 53, 182, 222, 158, 19, 104, 106, 15,
    ];

    /// A valid compressed G2 point, reused from the deprecated-API decompress test.
    const VALID_G2_COMPRESSED: [u8; 96] = [
        143, 150, 139, 210, 67, 144, 143, 243, 229, 250, 26, 179, 243, 30, 7, 129, 151, 229, 138,
        206, 86, 43, 190, 139, 90, 39, 29, 95, 186, 80, 35, 125, 160, 200, 254, 101, 231, 181, 119,
        28, 192, 168, 111, 213, 127, 50, 52, 126, 21, 162, 109, 31, 93, 86, 196, 114, 208, 25, 238,
        162, 83, 158, 88, 219, 0, 196, 154, 165, 208, 169, 102, 56, 56, 144, 63, 221, 190, 67, 107,
        91, 21, 126, 131, 179, 93, 26, 78, 95, 137, 247, 129, 39, 243, 93, 172, 240,
    ];

    #[test]
    fn map_fp_to_g1_success_and_error() {
        let points = bls12381::map_fp_to_g1(&[fp_one()]).unwrap();
        assert_eq!(points.len(), 1);
        // The convenience method agrees with the batch call.
        assert_eq!(fp_one().map_to_g1().unwrap(), points[0]);
        // A field element `>= p` (all `0xff`) is rejected instead of panicking.
        assert_eq!(bls12381::map_fp_to_g1(&[Fp([0xff; 48])]), Err(Error::InvalidInput));
    }

    #[test]
    fn map_fp2_to_g2_success() {
        let points = bls12381::map_fp2_to_g2(&[fp2_one()]).unwrap();
        assert_eq!(points.len(), 1);
        assert_eq!(fp2_one().map_to_g2().unwrap(), points[0]);
    }

    #[test]
    fn p1_sum_identity_and_empty() {
        let g1 = fp_one().map_to_g1().unwrap();
        // p + (-p) is the point at infinity.
        let id = bls12381::p1_sum(&[(Sign::Positive, g1), (Sign::Negative, g1)]).unwrap();
        assert_eq!(id, g1_identity());
        // The empty sum is also the identity.
        assert_eq!(bls12381::p1_sum(&[]).unwrap(), g1_identity());
    }

    #[test]
    fn p1_sum_rejects_garbage() {
        assert_eq!(bls12381::p1_sum(&[(Sign::Positive, G1([0xff; 96]))]), Err(Error::InvalidInput));
    }

    #[test]
    fn p2_sum_identity() {
        let g2 = fp2_one().map_to_g2().unwrap();
        let id = bls12381::p2_sum(&[(Sign::Positive, g2), (Sign::Negative, g2)]).unwrap();
        assert_eq!(id, g2_identity());
    }

    #[test]
    fn g1_multiexp_scalar_one_is_the_point() {
        let g1 = fp_one().map_to_g1().unwrap();
        let out = bls12381::g1_multiexp(&[(g1, scalar_one())]).unwrap();
        assert_eq!(out, g1);
    }

    #[test]
    fn g2_multiexp_scalar_one_is_the_point() {
        let g2 = fp2_one().map_to_g2().unwrap();
        let out = bls12381::g2_multiexp(&[(g2, scalar_one())]).unwrap();
        assert_eq!(out, g2);
    }

    #[test]
    fn g1_multiexp_rejects_garbage() {
        assert_eq!(
            bls12381::g1_multiexp(&[(G1([0xff; 96]), scalar_one())]),
            Err(Error::InvalidInput)
        );
    }

    #[test]
    fn sum_and_decompress_accept_points_outside_subgroup() {
        // `(4, y)` is on the curve but not in `G1` (its order is not the subgroup order). This
        // avoids `(0, ±2)`, which sum and decompress reject without the NEP-488 fix.
        let y = "0a989badd40d6212b33cffc3f3763e9bc760f988c9926b26da9dd85e928483446346b8ed00e1de5d5ea93e354abe706c";
        let mut bytes = [0u8; 96];
        bytes[47] = 4;
        bytes[48..].copy_from_slice(&hex::decode(y).unwrap());
        let outside = G1(bytes);
        // Compressed form: `x` with the compression flag set; the sign flag is clear because `y`
        // is the smaller of the two roots.
        let mut compressed = [0u8; 48];
        compressed[0] = 0x80;
        compressed[47] = 4;

        // Sum and decompress accept it...
        assert!(bls12381::p1_sum(&[(Sign::Positive, outside)]).is_ok());
        assert_eq!(G1Compressed(compressed).decompress(), Ok(outside));
        // ...while multiexp and the pairing check reject it.
        assert_eq!(bls12381::g1_multiexp(&[(outside, scalar_one())]), Err(Error::InvalidInput));
        let g2 = fp2_one().map_to_g2().unwrap();
        assert_eq!(bls12381::pairing_check(&[(outside, g2)]), Err(Error::InvalidInput));
    }

    #[test]
    fn g1_multiexp_with_unit_scalars_is_the_sum() {
        let points = bls12381::map_fp_to_g1(&[fp_one(), Fp([2; 48])]).unwrap();
        let sum = bls12381::p1_sum(&[(Sign::Positive, points[0]), (Sign::Positive, points[1])]);
        let multiexp =
            bls12381::g1_multiexp(&[(points[0], scalar_one()), (points[1], scalar_one())]);
        assert_eq!(multiexp, sum);
    }

    #[test]
    fn decompress_g1_batch_and_convenience() {
        let compressed = G1Compressed(VALID_G1_COMPRESSED);
        let batch = bls12381::p1_decompress(&[compressed]).unwrap();
        assert_eq!(batch.len(), 1);
        assert_eq!(compressed.decompress().unwrap(), batch[0]);
        assert_eq!(bls12381::p1_decompress(&[G1Compressed([0xff; 48])]), Err(Error::InvalidInput));
    }

    #[test]
    fn decompress_g2_batch_and_convenience() {
        let compressed = G2Compressed(VALID_G2_COMPRESSED);
        let batch = bls12381::p2_decompress(&[compressed]).unwrap();
        assert_eq!(batch.len(), 1);
        assert_eq!(compressed.decompress().unwrap(), batch[0]);
    }

    #[test]
    fn decompress_empty_is_empty() {
        assert!(bls12381::p1_decompress(&[]).unwrap().is_empty());
        assert!(bls12381::p2_decompress(&[]).unwrap().is_empty());
    }

    #[test]
    fn pairing_check_three_states() {
        let g1 = fp_one().map_to_g1().unwrap();
        let g2 = fp2_one().map_to_g2().unwrap();

        // The empty product equals the identity, so the check passes.
        assert_eq!(bls12381::pairing_check(&[]), Ok(true));

        // e(g1, g2) * e(-g1, g2) = e(O, g2) = 1, so this passes with real points and
        // exercises the `Sign::Negative` semantics of `p1_sum`.
        let neg_g1 = bls12381::p1_sum(&[(Sign::Negative, g1)]).unwrap();
        assert_eq!(bls12381::pairing_check(&[(g1, g2), (neg_g1, g2)]), Ok(true));

        // A single non-trivial pairing is (almost surely) not the identity: fails, not errors.
        assert_eq!(bls12381::pairing_check(&[(g1, g2)]), Ok(false));

        // Malformed points are an error, distinguishable from a failing check.
        assert_eq!(
            bls12381::pairing_check(&[(G1([0xff; 96]), G2([0xff; 192]))]),
            Err(Error::InvalidInput)
        );
    }

    #[test]
    fn pairing_check_known_valid_vector() {
        // A known-passing vector (2 pairs of 96-byte G1 followed by 192-byte G2), reused from
        // `env`'s `bls12381_pairing_valid_check` test.
        let flat = hex::decode("085fad8696122c8a421033164e6a71d9adb3882933beba2c14dcad9bfd4badb30b49306c59a7a7837b72e02993f5a4ad025871da31a9be44cd3a46365038ef6f3658fc65ff3064e348083b2de4d983c7436f486f6e9de272fa0db7dfa543656811f7dbc8c5b084e2daf685536a2d155d69c7683b811c840e4167a5c966bad4eebfdb757ef9caa63ffde16727fa5c15ac0b15a2802624e85d6987eb53a69714401adfd5ca5e6151a8e9c0790dfc4494ea77ad32b66e95da7f615ee2fe7b6594f00493deb2392b4159afc07b69000f9b097ecca94bf5a46cb13f95dabdd9a40a2e207c077059c821caa29a40930b4b757f11404dcfe5e92c69acdbf3667651d5adf6856956805693fb945d83c5cf158371536814442ff31d6ad1b834a4ab13ad9917f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb114d1d6855d545a8aa7d76c8cf2e21f267816aef1db507c96655b9d5caac42364e6f38ba0ecb751bad54dcd6b939c2ca0f968bd243908ff3e5fa1ab3f31e078197e58ace562bbe8b5a271d5fba50237da0c8fe65e7b5771cc0a86fd57f32347e15a26d1f5d56c472d019eea2539e58db00c49aa5d0a9663838903fddbe436b5b157e83b35d1a4e5f89f78127f35dacf005a2854c7f36818c137070d1342bba362b5d0c7daed605fcc739df577c33bd6ab6e07ab4a97beee81aa57c8d41f447440eeaf1f595b7b57457d7792b4bc14be74d0038f7ac3767a9c61fecaa02c3d07982c02995f22f66c05b8eb3b9facd5571").unwrap();
        let pairs: Vec<(G1, G2)> = flat
            .chunks_exact(288)
            .map(|c| (G1(c[..96].try_into().unwrap()), G2(c[96..288].try_into().unwrap())))
            .collect();
        assert_eq!(pairs.len(), 2);
        assert_eq!(bls12381::pairing_check(&pairs), Ok(true));
    }

    #[test]
    fn generators_match_the_standard_encoding() {
        // The standard compressed generators, as serialized by e.g. zkcrypto `bls12_381`.
        let g1 = "97f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb";
        let g2 = "93e02b6052719f607dacd3a088274f65596bd0d09920b61ab5da61bbdc7f5049334cf11213945d57e5ac7d055d042b7e024aa2b2f08f0a91260805272dc51051c6e47ad4fa403b02b4510b647ae3d1770bac0326a805bbefd48056c8c121bdb8";
        let g1 = G1Compressed::try_from(hex::decode(g1).unwrap().as_slice()).unwrap();
        let g2 = G2Compressed::try_from(hex::decode(g2).unwrap().as_slice()).unwrap();
        assert_eq!(g1.decompress(), Ok(G1::GENERATOR));
        assert_eq!(g2.decompress(), Ok(G2::GENERATOR));

        assert_eq!(bls12381::p1_sum(&[(Sign::Negative, G1::GENERATOR)]), Ok(G1::NEG_GENERATOR));
        assert_eq!(bls12381::p2_sum(&[(Sign::Negative, G2::GENERATOR)]), Ok(G2::NEG_GENERATOR));

        // Multiexp checks subgroup membership, so this also shows both are in the subgroup.
        assert_eq!(bls12381::g1_multiexp(&[(G1::GENERATOR, scalar_one())]), Ok(G1::GENERATOR));
        assert_eq!(bls12381::g2_multiexp(&[(G2::GENERATOR, scalar_one())]), Ok(G2::GENERATOR));
    }

    #[test]
    fn signature_check_with_neg_generator() {
        // The check from the module-level example, with a key pair made in the test: the public
        // key is `sk * g1` and the signature is `sk * H(m)`.
        let sk = Scalar([7; 32]);
        let hashed_message = fp2_one().map_to_g2().unwrap();
        let pubkey = bls12381::g1_multiexp(&[(G1::GENERATOR, sk)]).unwrap();
        let signature = bls12381::g2_multiexp(&[(hashed_message, sk)]).unwrap();
        assert_eq!(
            bls12381::pairing_check(&[(pubkey, hashed_message), (G1::NEG_GENERATOR, signature)]),
            Ok(true)
        );

        // A signature made with a different key fails the check.
        let forged = bls12381::g2_multiexp(&[(hashed_message, scalar_one())]).unwrap();
        assert_eq!(
            bls12381::pairing_check(&[(pubkey, hashed_message), (G1::NEG_GENERATOR, forged)]),
            Ok(false)
        );
    }

    #[test]
    fn try_from_slice_length_checks() {
        assert!(G1::try_from([0u8; 96].as_slice()).is_ok());
        assert_eq!(G1::try_from([0u8; 95].as_slice()), Err(Error::InvalidInput));
        assert_eq!(G2::try_from([0u8; 191].as_slice()), Err(Error::InvalidInput));
        assert_eq!(Scalar::try_from([0u8; 33].as_slice()), Err(Error::InvalidInput));
    }
}
