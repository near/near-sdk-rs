/* tslint:disable */
/* eslint-disable */
/**
* @param {any} wasm_bytes 
* @returns {any} 
*/
export function inject_contract(wasm_bytes: any): any;
/**
* @param {any} mem 
*/
export function test_memory(mem: any): void;
export class VM {
  free(): void;
/**
* @param {any} context 
* @param {any} mem 
*/
  constructor(context: any, mem: any);
/**
*/
  reset(): void;
/**
* @param {any} context 
*/
  set_context(context: any): void;
/**
* @param {any} s 
*/
  set_current_account_id(s: any): void;
/**
* @param {any} s 
*/
  set_input(s: any): void;
/**
* @param {any} s 
*/
  set_signer_account_id(s: any): void;
/**
* The public key that was used to sign the original transaction that led to
* this execution.
* @param {any} s 
*/
  set_signer_account_pk(s: any): void;
/**
* @param {any} s 
*/
  set_predecessor_account_id(s: any): void;
/**
* @param {BigInt} block_height 
*/
  set_block_index(block_height: BigInt): void;
/**
* @param {BigInt} stmp 
*/
  set_block_timestamp(stmp: BigInt): void;
/**
* @param {any} u_128 
*/
  set_account_balance(u_128: any): void;
/**
* @param {any} u_128 
*/
  set_account_locked_balance(u_128: any): void;
/**
* @param {BigInt} amt 
*/
  set_storage_usage(amt: BigInt): void;
/**
* @param {any} u_128 
*/
  set_attached_deposit(u_128: any): void;
/**
* @param {BigInt} _u64 
*/
  set_prepaid_gas(_u64: BigInt): void;
/**
* @param {any} s 
*/
  set_random_seed(s: any): void;
/**
* @param {boolean} b 
*/
  set_is_view(b: boolean): void;
/**
* @param {any} arr 
*/
  set_output_data_receivers(arr: any): void;
/**
* #################
* # Registers API #
* #################
* Writes the entire content from the register `register_id` into the memory of the guest starting with `ptr`.
*
* # Arguments
*
* * `register_id` -- a register id from where to read the data;
* * `ptr` -- location on guest memory where to copy the data.
*
* # Errors
*
* * If the content extends outside the memory allocated to the guest. In Wasmer, it returns `MemoryAccessViolation` error message;
* * If `register_id` is pointing to unused register returns `InvalidRegisterId` error message.
*
* # Undefined Behavior
*
* If the content of register extends outside the preallocated memory on the host side, or the pointer points to a
* wrong location this function will overwrite memory that it is not supposed to overwrite causing an undefined behavior.
*
* # Cost
*
* `base + read_register_base + read_register_byte * num_bytes + write_memory_base + write_memory_byte * num_bytes`
* @param {BigInt} register_id 
* @param {BigInt} ptr 
*/
  read_register(register_id: BigInt, ptr: BigInt): void;
/**
* @param {BigInt} register_id 
* @returns {BigInt} 
*/
  register_len(register_id: BigInt): BigInt;
/**
* ###################################
* # String reading helper functions #
* ###################################
* Helper function to read and return utf8-encoding string.
* If `len == u64::MAX` then treats the string as null-terminated with character `\'\\0\'`.
*
* # Errors
*
* * If string extends outside the memory of the guest with `MemoryAccessViolation`;
* * If string is not UTF-8 returns `BadUtf8`.
* * If string is longer than `max_log_len` returns `BadUtf8`.
*
* # Cost
*
* For not nul-terminated string:
* `read_memory_base + read_memory_byte * num_bytes + utf8_decoding_base + utf8_decoding_byte * num_bytes`
*
* For nul-terminated string:
* `(read_memory_base + read_memory_byte) * num_bytes + utf8_decoding_base + utf8_decoding_byte * num_bytes`
* Helper function to read UTF-16 formatted string from guest memory.
* # Errors
*
* * If string extends outside the memory of the guest with `MemoryAccessViolation`;
* * If string is not UTF-16 returns `BadUtf16`.
*
* # Cost
*
* For not nul-terminated string:
* `read_memory_base + read_memory_byte * num_bytes + utf16_decoding_base + utf16_decoding_byte * num_bytes`
*
* For nul-terminated string:
* `read_memory_base * num_bytes / 2 + read_memory_byte * num_bytes + utf16_decoding_base + utf16_decoding_byte * num_bytes`
* ###############
* # Context API #
* ###############
* Saves the account id of the current contract that we execute into the register.
*
* # Errors
*
* If the registers exceed the memory limit returns `MemoryAccessViolation`.
*
* # Cost
*
* `base + write_register_base + write_register_byte * num_bytes`
* @param {BigInt} register_id 
*/
  current_account_id(register_id: BigInt): void;
/**
* All contract calls are a result of some transaction that was signed by some account using
* some access key and submitted into a memory pool (either through the wallet using RPC or by
* a node itself). This function returns the id of that account. Saves the bytes of the signer
* account id into the register.
*
* # Errors
*
* * If the registers exceed the memory limit returns `MemoryAccessViolation`.
* * If called as view function returns `ProhibitedInView`.
*
* # Cost
*
* `base + write_register_base + write_register_byte * num_bytes`
* @param {BigInt} register_id 
*/
  signer_account_id(register_id: BigInt): void;
/**
* Saves the public key fo the access key that was used by the signer into the register. In
* rare situations smart contract might want to know the exact access key that was used to send
* the original transaction, e.g. to increase the allowance or manipulate with the public key.
*
* # Errors
*
* * If the registers exceed the memory limit returns `MemoryAccessViolation`.
* * If called as view function returns `ProhibitedInView`.
*
* # Cost
*
* `base + write_register_base + write_register_byte * num_bytes`
* @param {BigInt} register_id 
*/
  signer_account_pk(register_id: BigInt): void;
/**
* All contract calls are a result of a receipt, this receipt might be created by a transaction
* that does function invocation on the contract or another contract as a result of
* cross-contract call. Saves the bytes of the predecessor account id into the register.
*
* # Errors
*
* * If the registers exceed the memory limit returns `MemoryAccessViolation`.
* * If called as view function returns `ProhibitedInView`.
*
* # Cost
*
* `base + write_register_base + write_register_byte * num_bytes`
* @param {BigInt} register_id 
*/
  predecessor_account_id(register_id: BigInt): void;
/**
* Reads input to the contract call into the register. Input is expected to be in JSON-format.
* If input is provided saves the bytes (potentially zero) of input into register. If input is
* not provided writes 0 bytes into the register.
*
* # Cost
*
* `base + write_register_base + write_register_byte * num_bytes`
* @param {BigInt} register_id 
*/
  input(register_id: BigInt): void;
/**
* Returns the current block height.
*
* # Cost
*
* `base`
* TODO #1903 rename to `block_height`
* @returns {BigInt} 
*/
  block_index(): BigInt;
/**
* Returns the current block timestamp.
*
* # Cost
*
* `base`
* @returns {BigInt} 
*/
  block_timestamp(): BigInt;
/**
* Returns the number of bytes used by the contract if it was saved to the trie as of the
* invocation. This includes:
* * The data written with storage_* functions during current and previous execution;
* * The bytes needed to store the access keys of the given account.
* * The contract code size
* * A small fixed overhead for account metadata.
*
* # Cost
*
* `base`
* @returns {BigInt} 
*/
  storage_usage(): BigInt;
/**
* #################
* # Economics API #
* #################
* The current balance of the given account. This includes the attached_deposit that was
* attached to the transaction.
*
* # Cost
*
* `base + memory_write_base + memory_write_size * 16`
* @param {BigInt} balance_ptr 
*/
  account_balance(balance_ptr: BigInt): void;
/**
* The current amount of tokens locked due to staking.
*
* # Cost
*
* `base + memory_write_base + memory_write_size * 16`
* @param {BigInt} balance_ptr 
*/
  account_locked_balance(balance_ptr: BigInt): void;
/**
* The balance that was attached to the call that will be immediately deposited before the
* contract execution starts.
*
* # Errors
*
* If called as view function returns `ProhibitedInView``.
*
* # Cost
*
* `base + memory_write_base + memory_write_size * 16`
* @param {BigInt} balance_ptr 
*/
  attached_deposit(balance_ptr: BigInt): void;
/**
* The amount of gas attached to the call that can be used to pay for the gas fees.
*
* # Errors
*
* If called as view function returns `ProhibitedInView`.
*
* # Cost
*
* `base`
* @returns {BigInt} 
*/
  prepaid_gas(): BigInt;
/**
* The gas that was already burnt during the contract execution (cannot exceed `prepaid_gas`)
*
* # Errors
*
* If called as view function returns `ProhibitedInView`.
*
* # Cost
*
* `base`
* @returns {BigInt} 
*/
  used_gas(): BigInt;
/**
* ############
* # Math API #
* ############
* Writes random seed into the register.
*
* # Errors
*
* If the size of the registers exceed the set limit `MemoryAccessViolation`.
*
* # Cost
*
* `base + write_register_base + write_register_byte * num_bytes`.
* @param {BigInt} register_id 
*/
  random_seed(register_id: BigInt): void;
/**
* Hashes the random sequence of bytes using sha256 and returns it into `register_id`.
*
* # Errors
*
* If `value_len + value_ptr` points outside the memory or the registers use more memory than
* the limit with `MemoryAccessViolation`.
*
* # Cost
*
* `base + write_register_base + write_register_byte * num_bytes + sha256_base + sha256_byte * num_bytes`
* @param {BigInt} value_len 
* @param {BigInt} value_ptr 
* @param {BigInt} register_id 
*/
  sha256(value_len: BigInt, value_ptr: BigInt, register_id: BigInt): void;
/**
* Hashes the random sequence of bytes using keccak256 and returns it into `register_id`.
*
* # Errors
*
* If `value_len + value_ptr` points outside the memory or the registers use more memory than
* the limit with `MemoryAccessViolation`.
*
* # Cost
*
* `base + write_register_base + write_register_byte * num_bytes + keccak256_base + keccak256_byte * num_bytes`
* @param {BigInt} value_len 
* @param {BigInt} value_ptr 
* @param {BigInt} register_id 
*/
  keccak256(value_len: BigInt, value_ptr: BigInt, register_id: BigInt): void;
/**
* Hashes the random sequence of bytes using keccak512 and returns it into `register_id`.
*
* # Errors
*
* If `value_len + value_ptr` points outside the memory or the registers use more memory than
* the limit with `MemoryAccessViolation`.
*
* # Cost
*
* `base + write_register_base + write_register_byte * num_bytes + keccak512_base + keccak512_byte * num_bytes`
* @param {BigInt} value_len 
* @param {BigInt} value_ptr 
* @param {BigInt} register_id 
*/
  keccak512(value_len: BigInt, value_ptr: BigInt, register_id: BigInt): void;
/**
* Called by gas metering injected into Wasm. Counts both towards `burnt_gas` and `used_gas`.
*
* # Errors
*
* * If passed gas amount somehow overflows internal gas counters returns `IntegerOverflow`;
* * If we exceed usage limit imposed on burnt gas returns `GasLimitExceeded`;
* * If we exceed the `prepaid_gas` then returns `GasExceeded`.
* @param {number} gas_amount 
*/
  gas(gas_amount: number): void;
/**
* ################
* # Promises API #
* ################
* A helper function to pay gas fee for creating a new receipt without actions.
* # Args:
* * `sir`: whether contract call is addressed to itself;
* * `data_dependencies`: other contracts that this execution will be waiting on (or rather
*   their data receipts), where bool indicates whether this is sender=receiver communication.
*
* # Cost
*
* This is a convenience function that encapsulates several costs:
* `burnt_gas := dispatch cost of the receipt + base dispatch cost  cost of the data receipt`
* `used_gas := burnt_gas + exec cost of the receipt + base exec cost  cost of the data receipt`
* Notice that we prepay all base cost upon the creation of the data dependency, we are going to
* pay for the content transmitted through the dependency upon the actual creation of the
* DataReceipt.
* A helper function to subtract balance on transfer or attached deposit for promises.
* # Args:
* * `amount`: the amount to deduct from the current account balance.
* Creates a promise that will execute a method on account with given arguments and attaches
* the given amount and gas. `amount_ptr` point to slices of bytes representing `u128`.
*
* # Errors
*
* * If `account_id_len + account_id_ptr` or `method_name_len + method_name_ptr` or
* `arguments_len + arguments_ptr` or `amount_ptr + 16` points outside the memory of the guest
* or host returns `MemoryAccessViolation`.
* * If called as view function returns `ProhibitedInView`.
*
* # Returns
*
* Index of the new promise that uniquely identifies it within the current execution of the
* method.
*
* # Cost
*
* Since `promise_create` is a convenience wrapper around `promise_batch_create` and
* `promise_batch_action_function_call`. This also means it charges `base` cost twice.
* @param {BigInt} account_id_len 
* @param {BigInt} account_id_ptr 
* @param {BigInt} method_name_len 
* @param {BigInt} method_name_ptr 
* @param {BigInt} arguments_len 
* @param {BigInt} arguments_ptr 
* @param {BigInt} amount_ptr 
* @param {BigInt} gas 
* @returns {BigInt} 
*/
  promise_create(account_id_len: BigInt, account_id_ptr: BigInt, method_name_len: BigInt, method_name_ptr: BigInt, arguments_len: BigInt, arguments_ptr: BigInt, amount_ptr: BigInt, gas: BigInt): BigInt;
/**
* Attaches the callback that is executed after promise pointed by `promise_idx` is complete.
*
* # Errors
*
* * If `promise_idx` does not correspond to an existing promise returns `InvalidPromiseIndex`;
* * If `account_id_len + account_id_ptr` or `method_name_len + method_name_ptr` or
*   `arguments_len + arguments_ptr` or `amount_ptr + 16` points outside the memory of the
*   guest or host returns `MemoryAccessViolation`.
* * If called as view function returns `ProhibitedInView`.
*
* # Returns
*
* Index of the new promise that uniquely identifies it within the current execution of the
* method.
*
* # Cost
*
* Since `promise_create` is a convenience wrapper around `promise_batch_then` and
* `promise_batch_action_function_call`. This also means it charges `base` cost twice.
* @param {BigInt} promise_idx 
* @param {BigInt} account_id_len 
* @param {BigInt} account_id_ptr 
* @param {BigInt} method_name_len 
* @param {BigInt} method_name_ptr 
* @param {BigInt} arguments_len 
* @param {BigInt} arguments_ptr 
* @param {BigInt} amount_ptr 
* @param {BigInt} gas 
* @returns {BigInt} 
*/
  promise_then(promise_idx: BigInt, account_id_len: BigInt, account_id_ptr: BigInt, method_name_len: BigInt, method_name_ptr: BigInt, arguments_len: BigInt, arguments_ptr: BigInt, amount_ptr: BigInt, gas: BigInt): BigInt;
/**
* Creates a new promise which completes when time all promises passed as arguments complete.
* Cannot be used with registers. `promise_idx_ptr` points to an array of `u64` elements, with
* `promise_idx_count` denoting the number of elements. The array contains indices of promises
* that need to be waited on jointly.
*
* # Errors
*
* * If `promise_ids_ptr + 8 * promise_idx_count` extend outside the guest memory returns
*   `MemoryAccessViolation`;
* * If any of the promises in the array do not correspond to existing promises returns
*   `InvalidPromiseIndex`.
* * If called as view function returns `ProhibitedInView`.
*
* # Returns
*
* Index of the new promise that uniquely identifies it within the current execution of the
* method.
*
* # Cost
*
* `base + promise_and_base + promise_and_per_promise * num_promises + cost of reading promise ids from memory`.
* @param {BigInt} promise_idx_ptr 
* @param {BigInt} promise_idx_count 
* @returns {BigInt} 
*/
  promise_and(promise_idx_ptr: BigInt, promise_idx_count: BigInt): BigInt;
/**
* Creates a new promise towards given `account_id` without any actions attached to it.
*
* # Errors
*
* * If `account_id_len + account_id_ptr` points outside the memory of the guest or host
* returns `MemoryAccessViolation`.
* * If called as view function returns `ProhibitedInView`.
*
* # Returns
*
* Index of the new promise that uniquely identifies it within the current execution of the
* method.
*
* # Cost
*
* `burnt_gas := base + cost of reading and decoding the account id + dispatch cost of the receipt`.
* `used_gas := burnt_gas + exec cost of the receipt`.
* @param {BigInt} account_id_len 
* @param {BigInt} account_id_ptr 
* @returns {BigInt} 
*/
  promise_batch_create(account_id_len: BigInt, account_id_ptr: BigInt): BigInt;
/**
* Creates a new promise towards given `account_id` without any actions attached, that is
* executed after promise pointed by `promise_idx` is complete.
*
* # Errors
*
* * If `promise_idx` does not correspond to an existing promise returns `InvalidPromiseIndex`;
* * If `account_id_len + account_id_ptr` points outside the memory of the guest or host
* returns `MemoryAccessViolation`.
* * If called as view function returns `ProhibitedInView`.
*
* # Returns
*
* Index of the new promise that uniquely identifies it within the current execution of the
* method.
*
* # Cost
*
* `base + cost of reading and decoding the account id + dispatch&execution cost of the receipt
*  + dispatch&execution base cost for each data dependency`
* @param {BigInt} promise_idx 
* @param {BigInt} account_id_len 
* @param {BigInt} account_id_ptr 
* @returns {BigInt} 
*/
  promise_batch_then(promise_idx: BigInt, account_id_len: BigInt, account_id_ptr: BigInt): BigInt;
/**
* Appends `CreateAccount` action to the batch of actions for the given promise pointed by
* `promise_idx`.
*
* # Errors
*
* * If `promise_idx` does not correspond to an existing promise returns `InvalidPromiseIndex`.
* * If the promise pointed by the `promise_idx` is an ephemeral promise created by
* `promise_and` returns `CannotAppendActionToJointPromise`.
* * If called as view function returns `ProhibitedInView`.
*
* # Cost
*
* `burnt_gas := base + dispatch action fee`
* `used_gas := burnt_gas + exec action fee`
* @param {BigInt} promise_idx 
*/
  promise_batch_action_create_account(promise_idx: BigInt): void;
/**
* Appends `DeployContract` action to the batch of actions for the given promise pointed by
* `promise_idx`.
*
* # Errors
*
* * If `promise_idx` does not correspond to an existing promise returns `InvalidPromiseIndex`.
* * If the promise pointed by the `promise_idx` is an ephemeral promise created by
* `promise_and` returns `CannotAppendActionToJointPromise`.
* * If `code_len + code_ptr` points outside the memory of the guest or host returns
* `MemoryAccessViolation`.
* * If called as view function returns `ProhibitedInView`.
*
* # Cost
*
* `burnt_gas := base + dispatch action base fee + dispatch action per byte fee * num bytes + cost of reading vector from memory `
* `used_gas := burnt_gas + exec action base fee + exec action per byte fee * num bytes`
* @param {BigInt} promise_idx 
* @param {BigInt} code_len 
* @param {BigInt} code_ptr 
*/
  promise_batch_action_deploy_contract(promise_idx: BigInt, code_len: BigInt, code_ptr: BigInt): void;
/**
* Appends `FunctionCall` action to the batch of actions for the given promise pointed by
* `promise_idx`.
*
* # Errors
*
* * If `promise_idx` does not correspond to an existing promise returns `InvalidPromiseIndex`.
* * If the promise pointed by the `promise_idx` is an ephemeral promise created by
* `promise_and` returns `CannotAppendActionToJointPromise`.
* * If `method_name_len + method_name_ptr` or `arguments_len + arguments_ptr` or
* `amount_ptr + 16` points outside the memory of the guest or host returns
* `MemoryAccessViolation`.
* * If called as view function returns `ProhibitedInView`.
*
* # Cost
*
* `burnt_gas := base + dispatch action base fee + dispatch action per byte fee * num bytes + cost of reading vector from memory
*  + cost of reading u128, method_name and arguments from the memory`
* `used_gas := burnt_gas + exec action base fee + exec action per byte fee * num bytes`
* @param {BigInt} promise_idx 
* @param {BigInt} method_name_len 
* @param {BigInt} method_name_ptr 
* @param {BigInt} arguments_len 
* @param {BigInt} arguments_ptr 
* @param {BigInt} amount_ptr 
* @param {BigInt} gas 
*/
  promise_batch_action_function_call(promise_idx: BigInt, method_name_len: BigInt, method_name_ptr: BigInt, arguments_len: BigInt, arguments_ptr: BigInt, amount_ptr: BigInt, gas: BigInt): void;
/**
* Appends `Transfer` action to the batch of actions for the given promise pointed by
* `promise_idx`.
*
* # Errors
*
* * If `promise_idx` does not correspond to an existing promise returns `InvalidPromiseIndex`.
* * If the promise pointed by the `promise_idx` is an ephemeral promise created by
* `promise_and` returns `CannotAppendActionToJointPromise`.
* * If `amount_ptr + 16` points outside the memory of the guest or host returns
* `MemoryAccessViolation`.
* * If called as view function returns `ProhibitedInView`.
*
* # Cost
*
* `burnt_gas := base + dispatch action base fee + dispatch action per byte fee * num bytes + cost of reading u128 from memory `
* `used_gas := burnt_gas + exec action base fee + exec action per byte fee * num bytes`
* @param {BigInt} promise_idx 
* @param {BigInt} amount_ptr 
*/
  promise_batch_action_transfer(promise_idx: BigInt, amount_ptr: BigInt): void;
/**
* Appends `Stake` action to the batch of actions for the given promise pointed by
* `promise_idx`.
*
* # Errors
*
* * If `promise_idx` does not correspond to an existing promise returns `InvalidPromiseIndex`.
* * If the promise pointed by the `promise_idx` is an ephemeral promise created by
* `promise_and` returns `CannotAppendActionToJointPromise`.
* * If the given public key is not a valid (e.g. wrong length) returns `InvalidPublicKey`.
* * If `amount_ptr + 16` or `public_key_len + public_key_ptr` points outside the memory of the
* guest or host returns `MemoryAccessViolation`.
* * If called as view function returns `ProhibitedInView`.
*
* # Cost
*
* `burnt_gas := base + dispatch action base fee + dispatch action per byte fee * num bytes + cost of reading public key from memory `
* `used_gas := burnt_gas + exec action base fee + exec action per byte fee * num bytes`
* @param {BigInt} promise_idx 
* @param {BigInt} amount_ptr 
* @param {BigInt} public_key_len 
* @param {BigInt} public_key_ptr 
*/
  promise_batch_action_stake(promise_idx: BigInt, amount_ptr: BigInt, public_key_len: BigInt, public_key_ptr: BigInt): void;
/**
* Appends `AddKey` action to the batch of actions for the given promise pointed by
* `promise_idx`. The access key will have `FullAccess` permission.
*
* # Errors
*
* * If `promise_idx` does not correspond to an existing promise returns `InvalidPromiseIndex`.
* * If the promise pointed by the `promise_idx` is an ephemeral promise created by
* `promise_and` returns `CannotAppendActionToJointPromise`.
* * If the given public key is not a valid (e.g. wrong length) returns `InvalidPublicKey`.
* * If `public_key_len + public_key_ptr` points outside the memory of the guest or host
* returns `MemoryAccessViolation`.
* * If called as view function returns `ProhibitedInView`.
*
* # Cost
*
* `burnt_gas := base + dispatch action base fee + dispatch action per byte fee * num bytes + cost of reading public key from memory `
* `used_gas := burnt_gas + exec action base fee + exec action per byte fee * num bytes`
* @param {BigInt} promise_idx 
* @param {BigInt} public_key_len 
* @param {BigInt} public_key_ptr 
* @param {BigInt} nonce 
*/
  promise_batch_action_add_key_with_full_access(promise_idx: BigInt, public_key_len: BigInt, public_key_ptr: BigInt, nonce: BigInt): void;
/**
* Appends `AddKey` action to the batch of actions for the given promise pointed by
* `promise_idx`. The access key will have `FunctionCall` permission.
*
* # Errors
*
* * If `promise_idx` does not correspond to an existing promise returns `InvalidPromiseIndex`.
* * If the promise pointed by the `promise_idx` is an ephemeral promise created by
* `promise_and` returns `CannotAppendActionToJointPromise`.
* * If the given public key is not a valid (e.g. wrong length) returns `InvalidPublicKey`.
* * If `public_key_len + public_key_ptr`, `allowance_ptr + 16`,
* `receiver_id_len + receiver_id_ptr` or `method_names_len + method_names_ptr` points outside
* the memory of the guest or host returns `MemoryAccessViolation`.
* * If called as view function returns `ProhibitedInView`.
*
* # Cost
*
* `burnt_gas := base + dispatch action base fee + dispatch action per byte fee * num bytes + cost of reading vector from memory
*  + cost of reading u128, method_names and public key from the memory + cost of reading and parsing account name`
* `used_gas := burnt_gas + exec action base fee + exec action per byte fee * num bytes`
* @param {BigInt} promise_idx 
* @param {BigInt} public_key_len 
* @param {BigInt} public_key_ptr 
* @param {BigInt} nonce 
* @param {BigInt} allowance_ptr 
* @param {BigInt} receiver_id_len 
* @param {BigInt} receiver_id_ptr 
* @param {BigInt} method_names_len 
* @param {BigInt} method_names_ptr 
*/
  promise_batch_action_add_key_with_function_call(promise_idx: BigInt, public_key_len: BigInt, public_key_ptr: BigInt, nonce: BigInt, allowance_ptr: BigInt, receiver_id_len: BigInt, receiver_id_ptr: BigInt, method_names_len: BigInt, method_names_ptr: BigInt): void;
/**
* Appends `DeleteKey` action to the batch of actions for the given promise pointed by
* `promise_idx`.
*
* # Errors
*
* * If `promise_idx` does not correspond to an existing promise returns `InvalidPromiseIndex`.
* * If the promise pointed by the `promise_idx` is an ephemeral promise created by
* `promise_and` returns `CannotAppendActionToJointPromise`.
* * If the given public key is not a valid (e.g. wrong length) returns `InvalidPublicKey`.
* * If `public_key_len + public_key_ptr` points outside the memory of the guest or host
* returns `MemoryAccessViolation`.
* * If called as view function returns `ProhibitedInView`.
*
* # Cost
*
* `burnt_gas := base + dispatch action base fee + dispatch action per byte fee * num bytes + cost of reading public key from memory `
* `used_gas := burnt_gas + exec action base fee + exec action per byte fee * num bytes`
* @param {BigInt} promise_idx 
* @param {BigInt} public_key_len 
* @param {BigInt} public_key_ptr 
*/
  promise_batch_action_delete_key(promise_idx: BigInt, public_key_len: BigInt, public_key_ptr: BigInt): void;
/**
* Appends `DeleteAccount` action to the batch of actions for the given promise pointed by
* `promise_idx`.
*
* # Errors
*
* * If `promise_idx` does not correspond to an existing promise returns `InvalidPromiseIndex`.
* * If the promise pointed by the `promise_idx` is an ephemeral promise created by
* `promise_and` returns `CannotAppendActionToJointPromise`.
* * If `beneficiary_id_len + beneficiary_id_ptr` points outside the memory of the guest or
* host returns `MemoryAccessViolation`.
* * If called as view function returns `ProhibitedInView`.
*
* # Cost
*
* `burnt_gas := base + dispatch action base fee + dispatch action per byte fee * num bytes + cost of reading and parsing account id from memory `
* `used_gas := burnt_gas + exec action base fee + exec action per byte fee * num bytes`
* @param {BigInt} promise_idx 
* @param {BigInt} beneficiary_id_len 
* @param {BigInt} beneficiary_id_ptr 
*/
  promise_batch_action_delete_account(promise_idx: BigInt, beneficiary_id_len: BigInt, beneficiary_id_ptr: BigInt): void;
/**
* If the current function is invoked by a callback we can access the execution results of the
* promises that caused the callback. This function returns the number of complete and
* incomplete callbacks.
*
* Note, we are only going to have incomplete callbacks once we have promise_or combinator.
*
*
* * If there is only one callback returns `1`;
* * If there are multiple callbacks (e.g. created through `promise_and`) returns their number;
* * If the function was called not through the callback returns `0`.
*
* # Cost
*
* `base`
* @returns {BigInt} 
*/
  promise_results_count(): BigInt;
/**
* If the current function is invoked by a callback we can access the execution results of the
* promises that caused the callback. This function returns the result in blob format and
* places it into the register.
*
* * If promise result is complete and successful copies its blob into the register;
* * If promise result is complete and failed or incomplete keeps register unused;
*
* # Returns
*
* * If promise result is not complete returns `0`;
* * If promise result is complete and successful returns `1`;
* * If promise result is complete and failed returns `2`.
*
* # Errors
*
* * If `result_id` does not correspond to an existing result returns `InvalidPromiseResultIndex`;
* * If copying the blob exhausts the memory limit it returns `MemoryAccessViolation`.
* * If called as view function returns `ProhibitedInView`.
*
* # Cost
*
* `base + cost of writing data into a register`
* @param {BigInt} result_idx 
* @param {BigInt} register_id 
* @returns {BigInt} 
*/
  promise_result(result_idx: BigInt, register_id: BigInt): BigInt;
/**
* When promise `promise_idx` finishes executing its result is considered to be the result of
* the current function.
*
* # Errors
*
* * If `promise_idx` does not correspond to an existing promise returns `InvalidPromiseIndex`.
* * If called as view function returns `ProhibitedInView`.
*
* # Cost
*
* `base + promise_return`
* @param {BigInt} promise_idx 
*/
  promise_return(promise_idx: BigInt): void;
/**
* #####################
* # Miscellaneous API #
* #####################
* sets the blob of data as the return value of the contract.
*
* # Errors
*
* If `value_len + value_ptr` exceeds the memory container or points to an unused register it
* returns `MemoryAccessViolation`.
*
* # Cost
* `base + cost of reading return value from memory or register + dispatch&exec cost per byte of the data sent * num data receivers`
* @param {BigInt} value_len 
* @param {BigInt} value_ptr 
*/
  value_return(value_len: BigInt, value_ptr: BigInt): void;
/**
* Terminates the execution of the program with panic `GuestPanic`.
*
* # Cost
*
* `base`
*/
  panic(): void;
/**
* Guest panics with the UTF-8 encoded string.
* If `len == u64::MAX` then treats the string as null-terminated with character `\'\\0\'`.
*
* # Errors
*
* * If string extends outside the memory of the guest with `MemoryAccessViolation`;
* * If string is not UTF-8 returns `BadUtf8`.
* * If string is longer than `max_log_len` returns `BadUtf8`.
*
* # Cost
* `base + cost of reading and decoding a utf8 string`
* @param {BigInt} len 
* @param {BigInt} ptr 
*/
  panic_utf8(len: BigInt, ptr: BigInt): void;
/**
* Logs the UTF-8 encoded string.
* If `len == u64::MAX` then treats the string as null-terminated with character `\'\\0\'`.
*
* # Errors
*
* * If string extends outside the memory of the guest with `MemoryAccessViolation`;
* * If string is not UTF-8 returns `BadUtf8`.
* * If string is longer than `max_log_len` returns `BadUtf8`.
*
* # Cost
*
* `base + log_base + log_byte + num_bytes + utf8 decoding cost`
* @param {BigInt} len 
* @param {BigInt} ptr 
*/
  log_utf8(len: BigInt, ptr: BigInt): void;
/**
* Logs the UTF-16 encoded string. If `len == u64::MAX` then treats the string as
* null-terminated with two-byte sequence of `0x00 0x00`.
*
* # Errors
*
* * If string extends outside the memory of the guest with `MemoryAccessViolation`;
* * If string is not UTF-16 returns `BadUtf16`.
*
* # Cost
*
* `base + log_base + log_byte * num_bytes + utf16 decoding cost`
* @param {BigInt} len 
* @param {BigInt} ptr 
*/
  log_utf16(len: BigInt, ptr: BigInt): void;
/**
* Special import kept for compatibility with AssemblyScript contracts. Not called by smart
* contracts directly, but instead called by the code generated by AssemblyScript.
*
* # Cost
*
* `base +  log_base + log_byte * num_bytes + utf16 decoding cost`
* @param {number} msg_ptr 
* @param {number} filename_ptr 
* @param {number} line 
* @param {number} col 
*/
  abort(msg_ptr: number, filename_ptr: number, line: number, col: number): void;
/**
* ###############
* # Storage API #
* ###############
* Reads account id from the given location in memory.
*
* # Errors
*
* * If account is not UTF-8 encoded then returns `BadUtf8`;
*
* # Cost
*
* This is a helper function that encapsulates the following costs:
* cost of reading buffer from register or memory,
* `utf8_decoding_base + utf8_decoding_byte * num_bytes`.
* Writes key-value into storage.
* * If key is not in use it inserts the key-value pair and does not modify the register. Returns `0`;
* * If key is in use it inserts the key-value and copies the old value into the `register_id`. Returns `1`.
*
* # Errors
*
* * If `key_len + key_ptr` or `value_len + value_ptr` exceeds the memory container or points
*   to an unused register it returns `MemoryAccessViolation`;
* * If returning the preempted value into the registers exceed the memory container it returns
*   `MemoryAccessViolation`.
*
* # Cost
*
* `base + storage_write_base + storage_write_key_byte * num_key_bytes + storage_write_value_byte * num_value_bytes
* + get_vec_from_memory_or_register_cost x 2`.
*
* If a value was evicted it costs additional `storage_write_value_evicted_byte * num_evicted_bytes + internal_write_register_cost`.
* @param {BigInt} key_len 
* @param {BigInt} key_ptr 
* @param {BigInt} value_len 
* @param {BigInt} value_ptr 
* @param {BigInt} register_id 
* @returns {BigInt} 
*/
  storage_write(key_len: BigInt, key_ptr: BigInt, value_len: BigInt, value_ptr: BigInt, register_id: BigInt): BigInt;
/**
* Reads the value stored under the given key.
* * If key is used copies the content of the value into the `register_id`, even if the content
*   is zero bytes. Returns `1`;
* * If key is not present then does not modify the register. Returns `0`;
*
* # Errors
*
* * If `key_len + key_ptr` exceeds the memory container or points to an unused register it
*   returns `MemoryAccessViolation`;
* * If returning the preempted value into the registers exceed the memory container it returns
*   `MemoryAccessViolation`.
*
* # Cost
*
* `base + storage_read_base + storage_read_key_byte * num_key_bytes + storage_read_value_byte + num_value_bytes
*  cost to read key from register + cost to write value into register`.
* @param {BigInt} key_len 
* @param {BigInt} key_ptr 
* @param {BigInt} register_id 
* @returns {BigInt} 
*/
  storage_read(key_len: BigInt, key_ptr: BigInt, register_id: BigInt): BigInt;
/**
* Removes the value stored under the given key.
* * If key is used, removes the key-value from the trie and copies the content of the value
*   into the `register_id`, even if the content is zero bytes. Returns `1`;
* * If key is not present then does not modify the register. Returns `0`.
*
* # Errors
*
* * If `key_len + key_ptr` exceeds the memory container or points to an unused register it
*   returns `MemoryAccessViolation`;
* * If the registers exceed the memory limit returns `MemoryAccessViolation`;
* * If returning the preempted value into the registers exceed the memory container it returns
*   `MemoryAccessViolation`.
*
* # Cost
*
* `base + storage_remove_base + storage_remove_key_byte * num_key_bytes + storage_remove_ret_value_byte * num_value_bytes
* + cost to read the key + cost to write the value`.
* @param {BigInt} key_len 
* @param {BigInt} key_ptr 
* @param {BigInt} register_id 
* @returns {BigInt} 
*/
  storage_remove(key_len: BigInt, key_ptr: BigInt, register_id: BigInt): BigInt;
/**
* Checks if there is a key-value pair.
* * If key is used returns `1`, even if the value is zero bytes;
* * Otherwise returns `0`.
*
* # Errors
*
* If `key_len + key_ptr` exceeds the memory container it returns `MemoryAccessViolation`.
*
* # Cost
*
* `base + storage_has_key_base + storage_has_key_byte * num_bytes + cost of reading key`
* @param {BigInt} key_len 
* @param {BigInt} key_ptr 
* @returns {BigInt} 
*/
  storage_has_key(key_len: BigInt, key_ptr: BigInt): BigInt;
/**
* Utilities
*Computes the outcome of execution.
* @returns {any} 
*/
  outcome(): any;
/**
* @returns {any} 
*/
  created_receipts(): any;
}
