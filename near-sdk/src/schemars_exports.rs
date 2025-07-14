#[cfg(feature = "schemars-v1")]
pub use schemars_v1 as schemars;

#[cfg(feature = "schemars-v0_8")]
pub use schemars_v0_8 as schemars;

#[cfg(all(feature = "schemars-v1", feature = "schemars-v0_8"))]
compile_error!("features schemars-v1 and schemars-v0_8 are not allowed to be enabled at the same time. Only 1 of them is allowed");