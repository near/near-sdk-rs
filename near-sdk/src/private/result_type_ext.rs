pub trait ResultTypeExt: seal::ResultTypeExtSeal {
    type Okay;
    type Error;
}

impl<T, E> ResultTypeExt for Result<T, E> {
    type Okay = T;
    type Error = E;
}

// This is the "sealed trait" pattern:
// https://rust-lang.github.io/api-guidelines/future-proofing.html
mod seal {
    pub trait ResultTypeExtSeal {}

    impl<T, E> ResultTypeExtSeal for Result<T, E> {}
}
