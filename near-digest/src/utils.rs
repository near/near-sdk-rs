use std::marker::PhantomData;

use digest::{FixedOutput, FixedOutputReset, HashMarker, Output, OutputSizeUser, Reset, Update};
use impl_tools::autoimpl;

pub trait DigestFinalizer: OutputSizeUser {
    fn digest(bytes: &[u8]) -> Output<Self>;
}

#[cfg_attr(feature = "zeroize", derive(::zeroize::ZeroizeOnDrop))]
#[autoimpl(Debug, Clone, Default)]
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

impl<F> Reset for DigestFn<F> {
    #[inline]
    fn reset(&mut self) {
        self.data.clear();
    }
}

impl<F: DigestFinalizer> FixedOutputReset for DigestFn<F> {
    #[inline]
    fn finalize_into_reset(&mut self, out: &mut digest::Output<Self>) {
        *out = F::digest(&self.data);
        self.data.clear();
    }
}

impl<F: DigestFinalizer> HashMarker for DigestFn<F> {}
