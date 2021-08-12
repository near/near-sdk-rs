mod external;
mod mocked_blockchain;
mod receipt;

pub(crate) use self::external::SdkExternal;
pub use self::mocked_blockchain::MockedBlockchain;
pub use self::receipt::{Receipt, VmAction};
use core::cell::RefCell;

thread_local! {
    /// Low-level blockchain interface wrapped by the environment. Prefer using `env::*` and
    /// `testing_env` for interacting with the real and fake blockchains.
    static BLOCKCHAIN_INTERFACE: RefCell<MockedBlockchain>
         = RefCell::new(MockedBlockchain::default());
}

/// Perform function on a mutable reference to the [`MockedBlockchain`]. This can only be used
/// inside tests.
pub fn with_mocked_blockchain<F, R>(f: F) -> R
where
    F: FnOnce(&mut MockedBlockchain) -> R,
{
    BLOCKCHAIN_INTERFACE.with(|b| f(&mut b.borrow_mut()))
}
