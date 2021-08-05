use crate::{Balance, Gas, Promise, PromiseOrValue};

pub struct FunctionCallBuilder {
    promise: Option<Promise>,
    method_name: String,
    args: Vec<u8>,
    amount: Balance,
    gas: Option<Gas>,
}

impl FunctionCallBuilder {
    pub fn new(promise: Promise, method_name: String, args: Vec<u8>) -> Self {
        Self { promise: Some(promise), method_name, args, amount: 0, gas: None }
    }

    pub fn into_promise(mut self) -> Promise {
        self.construct_promise()
    }

    fn construct_promise(&mut self) -> Promise {
        let method_name = core::mem::take(&mut self.method_name);
        let arguments = core::mem::take(&mut self.args);
        let gas = core::mem::take(&mut self.gas);

        core::mem::take(&mut self.promise)
            .map(|p| p.queued_function_call(method_name, arguments, self.amount, gas))
            // Promise guaranteed to be Some unless promise is constructed, and only constructed
            // when dropped
            .unwrap_or_else(|| unreachable!())
    }

    pub fn with_amount(mut self, amount: Balance) -> Self {
        self.amount = amount;
        self
    }

    pub fn with_gas(mut self, gas: Gas) -> Self {
        self.gas = Some(gas);
        self
    }
}

impl From<FunctionCallBuilder> for Promise {
    fn from(mut f: FunctionCallBuilder) -> Self {
        f.construct_promise()
    }
}

impl<T> From<FunctionCallBuilder> for PromiseOrValue<T> {
    fn from(mut f: FunctionCallBuilder) -> Self {
        PromiseOrValue::Promise(f.construct_promise())
    }
}

impl Drop for FunctionCallBuilder {
    fn drop(&mut self) {
        if self.promise.is_some() {
            self.construct_promise();
        }
    }
}
