use std::marker::PhantomData;

use digest::{FixedOutput, HashMarker, Output, OutputSizeUser, Update};
use impl_tools::autoimpl;

pub trait DigestFinalizer: OutputSizeUser {
    fn digest(bytes: &[u8]) -> Output<Self>;
}

#[cfg_attr(feature = "zeroize", derive(::zeroize::ZeroizeOnDrop))]
#[autoimpl(Debug, Clone, Default, PartialEq, Eq)]
pub struct DigestFn<F> {
    data: Vec<u8>,
    _fn: PhantomData<F>,
}

impl<F: OutputSizeUser> OutputSizeUser for DigestFn<F> {
    type OutputSize = F::OutputSize;
}

impl<F> Update for DigestFn<F> {
    #[inline]
    fn update(&mut self, data: &[u8]) {
        self.data.extend(data);
    }
}

impl<F: DigestFinalizer> FixedOutput for DigestFn<F> {
    #[inline]
    fn finalize_into(self, out: &mut digest::Output<Self>) {
        *out = F::digest(&self.data);
    }
}

impl<F: DigestFinalizer> HashMarker for DigestFn<F> {}
