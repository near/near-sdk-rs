use borsh::BorshSerialize;

use super::Vector;

impl<T> Drop for Vector<T>
where
    T: BorshSerialize,
{
    fn drop(&mut self) {
        self.flush()
    }
}

// TODO index/indexmut

// TODO iter/double ended iter

// TODO iterator impls: from, into, extend
