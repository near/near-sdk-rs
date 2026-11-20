use near_sdk::{
    AccountId, CryptoHash, Gas, GasWeight, NearToken, PromiseError, env, log, near, serde_json,
    store::IterableMap,
};

// Register used to receive data id from `promise_await_data`.
const DATA_ID_REGISTER: u64 = 0;

// Prepaid gas for a `sign_on_finish` call
const SIGN_ON_FINISH_CALL_GAS: Gas = Gas::from_tgas(5);

// Prepaid gas for a `do_something` call
const CHAINED_CALL_GAS: Gas = Gas::from_tgas(5);

/// Deterministically derive a 32-byte yield id from a request index. Both `sign_with_id` and
/// `sign_respond_with_id` derive it the same way, so the responder can recompute the id from the
/// public request index instead of needing a runtime-generated token handed back to it. A
/// production contract would typically fold in more context (e.g. the signer and message) to keep
/// ids unique across unrelated requests.
fn derive_yield_id(index: u64) -> CryptoHash {
    env::sha256_array(&index.to_le_bytes())
}

#[near(serializers = [borsh, json])]
#[derive(Clone, Debug)]
pub struct SignatureRequest {
    pub data_id: CryptoHash,
    pub account_id: AccountId,
    pub message: String,
}

#[near(contract_state)]
pub struct MpcContract {
    requests: IterableMap<u64, SignatureRequest>,
    next_available_request_index: u64,
}

impl Default for MpcContract {
    fn default() -> Self {
        Self {
            requests: IterableMap::new(b"requests".to_vec()),
            next_available_request_index: 0u64,
        }
    }
}

#[near]
impl MpcContract {
    /// User-facing API: accepts some message and returns a signature
    pub fn sign(&mut self, message: String) {
        let index = self.next_available_request_index;
        self.next_available_request_index += 1;

        let yield_promise = env::promise_yield_create(
            "sign_on_finish",
            &serde_json::to_vec(&(index,)).unwrap(),
            SIGN_ON_FINISH_CALL_GAS,
            GasWeight(0),
            DATA_ID_REGISTER,
        );

        // Store the request in the contract's local state
        let data_id: CryptoHash =
            env::read_register(DATA_ID_REGISTER).expect("").try_into().expect("");
        self.requests.insert(
            index,
            SignatureRequest { data_id, account_id: env::signer_account_id(), message },
        );

        // The yield promise is composable with the usual promise API features. We can choose to
        // chain another function call and it will receive the output of the `sign_on_finish`
        // callback. Note that this chained promise can be a cross-contract call.
        env::promise_then(
            yield_promise,
            env::current_account_id(),
            "do_something",
            &[],
            NearToken::from_near(0),
            CHAINED_CALL_GAS,
        );

        // The return value for this function call will be the value
        // returned by the `sign_on_finish` callback.
        env::promise_return(yield_promise);
    }

    /// Called by MPC participants to submit a signature
    pub fn sign_respond(&mut self, data_id: CryptoHash, signature: String) {
        // check that caller is allowed to respond, signature is valid, etc.
        // ...

        log!("submitting response {} for data id {:?}", &signature, &data_id);
        env::promise_yield_resume(&data_id, &serde_json::to_vec(&signature).unwrap());
    }

    /// Variant of [`Self::sign`] using nearcore 2.13's `promise_yield_create_with_id`. Instead of
    /// letting the runtime generate the resumption token, the contract derives the yield id itself
    /// (here, from the request index). Because the id is deterministic, the responder can recompute
    /// it from the public request index alone (see `sign_respond_with_id`), so the contract never
    /// has to read a `data_id` out of a register and surface it.
    ///
    /// The high-level equivalents are `near_sdk::Promise::new_yield_with_id` and
    /// `near_sdk::UserYieldId`.
    pub fn sign_with_id(&mut self, message: String) {
        let index = self.next_available_request_index;
        self.next_available_request_index += 1;

        let yield_id = derive_yield_id(index);

        // `promise_yield_create_with_id` returns `None` if a yield with this id is already pending
        // for this account. Deriving from the unique request index means that can't happen here,
        // but a real contract can branch on `None` to recover from duplicates instead of aborting.
        let yield_promise = env::promise_yield_create_with_id(
            "sign_on_finish",
            &serde_json::to_vec(&(index,)).unwrap(),
            NearToken::from_near(0),
            SIGN_ON_FINISH_CALL_GAS,
            GasWeight(0),
            &yield_id,
        )
        .unwrap_or_else(|| env::panic_str("a request with this yield id is already pending"));

        // We can still store the id for `log_pending_requests`/`get_requests`, but note the
        // responder does not depend on it — it recomputes the id from `index`.
        self.requests.insert(
            index,
            SignatureRequest { data_id: yield_id, account_id: env::signer_account_id(), message },
        );

        // Composable with the rest of the promise API, exactly like `sign`.
        env::promise_then(
            yield_promise,
            env::current_account_id(),
            "do_something",
            &[],
            NearToken::from_near(0),
            CHAINED_CALL_GAS,
        );

        env::promise_return(yield_promise);
    }

    /// Called by MPC participants to submit a signature for a request created with
    /// [`Self::sign_with_id`]. The responder only needs the public `request_index`: it recomputes
    /// the same yield id and resumes by id, without the contract surfacing a runtime `data_id`.
    pub fn sign_respond_with_id(&mut self, request_index: u64, signature: String) {
        // check that caller is allowed to respond, signature is valid, etc.
        // ...

        let yield_id = derive_yield_id(request_index);
        log!("submitting response {} for request {}", &signature, request_index);
        env::promise_yield_resume_with_yield_id(
            &yield_id,
            &serde_json::to_vec(&signature).unwrap(),
        );
    }

    /// Callback receiving the externally submitted data (or a PromiseError)
    pub fn sign_on_finish(
        &mut self,
        request_index: u64,
        #[callback_result] signature: Result<String, PromiseError>,
    ) -> String {
        // Clean up the local state
        self.requests.remove(&request_index);

        match signature {
            Ok(signature) => "signature received: ".to_owned() + &signature,
            Err(_) => "signature request timed out".to_string(),
        }
    }

    pub fn do_something(#[callback_unwrap] signature_result: String) {
        log!("fn do_something invoked with result '{}'", &signature_result);
    }

    /// Helper for local testing; prints all pending requests
    pub fn log_pending_requests(&self) {
        for (_, request) in self.requests.iter() {
            log!(
                "{}: account_id={} payload={}",
                hex::encode(request.data_id),
                request.account_id,
                request.message
            );
        }
    }

    pub fn get_requests(&self) -> Vec<SignatureRequest> {
        self.requests.iter().map(|(_, request)| request).cloned().collect()
    }
}
