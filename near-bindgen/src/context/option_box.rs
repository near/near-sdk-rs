//! A combination of `Box` and `Option` that allows setting `context` after launch.

use std::cell::RefCell;
use std::ops::Deref;

pub struct BoxOption<T: ?Sized> {
    content: RefCell<Option<Box<T>>>,
}

impl<T: ?Sized> BoxOption<T> {
    pub fn new() -> Self {
        Self { content: RefCell::new(None) }
    }

    pub fn set(&self, content: Box<T>) {
        *self.content.borrow_mut() = Some(content);
    }
}

impl<T: ?Sized> Deref for BoxOption<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        let ptr = self.content.borrow().as_ref().unwrap().as_ref() as *const T;
        unsafe { &*ptr }
    }
}

// We are not supposed to use threads anyway.
unsafe impl<T: ?Sized> Sync for BoxOption<T> {}
unsafe impl<T: ?Sized> Send for BoxOption<T> {}
