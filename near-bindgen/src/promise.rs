use near_vm_logic::types::{AccountId, Balance, Gas, PromiseIndex};
use std::cell::RefCell;
use std::rc::Rc;

pub struct PromiseSingle {
    pub account_id: AccountId,
    pub method_name: Vec<u8>,
    pub args: Vec<u8>,
    pub balance: Balance,
    pub gas: Gas,
    pub after: RefCell<Option<Promise>>,
    /// Promise index that is computed only once.
    pub promise_index: RefCell<Option<PromiseIndex>>,
}

impl PromiseSingle {
    pub fn construct_recursively(&self) -> PromiseIndex {
        let mut promise_lock = self.promise_index.borrow_mut();
        if let Some(res) = promise_lock.as_ref() {
            return *res;
        }
        let res = if let Some(after) = self.after.borrow().as_ref() {
            crate::env::promise_then(
                after.construct_recursively(),
                self.account_id.clone(),
                &self.method_name,
                &self.args,
                self.balance,
                self.gas,
            )
        } else {
            crate::env::promise_create(
                self.account_id.clone(),
                &self.method_name,
                &self.args,
                self.balance,
                self.gas,
            )
        };
        *promise_lock = Some(res);
        res
    }
}

pub struct PromiseJoint {
    pub promise_a: Promise,
    pub promise_b: Promise,
    /// Promise index that is computed only once.
    pub promise_index: RefCell<Option<PromiseIndex>>,
}

impl PromiseJoint {
    pub fn construct_recursively(&self) -> PromiseIndex {
        let mut promise_lock = self.promise_index.borrow_mut();
        if let Some(res) = promise_lock.as_ref() {
            return *res;
        }
        let res = crate::env::promise_and(&[
            self.promise_a.construct_recursively(),
            self.promise_b.construct_recursively(),
        ]);
        *promise_lock = Some(res);
        res
    }
}

#[derive(Clone)]
pub struct Promise {
    pub subtype: PromiseSubtype,
    pub should_return: bool,
}

#[derive(Clone)]
pub enum PromiseSubtype {
    Single(Rc<PromiseSingle>),
    Joint(Rc<PromiseJoint>),
}

impl Promise {
    pub fn new(
        account_id: AccountId,
        method_name: Vec<u8>,
        args: Vec<u8>,
        balance: Balance,
        gas: Gas,
        after: Option<Self>,
    ) -> Self {
        Self {
            subtype: PromiseSubtype::Single(Rc::new(PromiseSingle {
                account_id,
                method_name,
                args,
                balance,
                gas,
                after: RefCell::new(after),
                promise_index: RefCell::new(None),
            })),
            should_return: false,
        }
    }

    pub fn join(self, other: Promise) -> Promise {
        Promise {
            subtype: PromiseSubtype::Joint(Rc::new(PromiseJoint {
                promise_a: self,
                promise_b: other,
                promise_index: RefCell::new(None),
            })),
            should_return: false,
        }
    }

    pub fn and_then(self, mut other: Promise) -> Promise {
        match &mut other.subtype {
            PromiseSubtype::Single(x) => *x.after.borrow_mut() = Some(self),
            PromiseSubtype::Joint(_) => panic!("Cannot callback joint promise."),
        }
        other
    }

    pub fn as_return(mut self) -> Self {
        self.should_return = true;
        self
    }

    fn construct_recursively(&self) -> PromiseIndex {
        let res = match &self.subtype {
            PromiseSubtype::Single(x) => x.construct_recursively(),
            PromiseSubtype::Joint(x) => x.construct_recursively(),
        };
        if self.should_return {
            crate::env::promise_return(res);
        }
        res
    }
}

impl Drop for Promise {
    fn drop(&mut self) {
        self.construct_recursively();
    }
}

pub enum PromiseOrValue<T> {
    Promise(Promise),
    Value(T),
}

impl<T> From<Promise> for PromiseOrValue<T> {
    fn from(promise: Promise) -> Self {
        PromiseOrValue::Promise(promise.as_return())
    }
}

impl<T: serde::Serialize> serde::Serialize for PromiseOrValue<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            // Only actual value is serialized.
            PromiseOrValue::Value(x) => x.serialize(serializer),
            // The promise is dropped to cause env::promise calls.
            PromiseOrValue::Promise(_) => serializer.serialize_unit(),
        }
    }
}
