let imports = {};
imports['__wbindgen_placeholder__'] = module.exports;
let wasm;
const { TextDecoder } = require(String.raw`util`);

const heap = new Array(32).fill(undefined);

heap.push(undefined, null, true, false);

function getObject(idx) { return heap[idx]; }

let heap_next = heap.length;

function addHeapObject(obj) {
    if (heap_next === heap.length) heap.push(heap.length + 1);
    const idx = heap_next;
    heap_next = heap[idx];

    if (typeof(heap_next) !== 'number') throw new Error('corrupt heap');

    heap[idx] = obj;
    return idx;
}

function dropObject(idx) {
    if (idx < 36) return;
    heap[idx] = heap_next;
    heap_next = idx;
}

function takeObject(idx) {
    const ret = getObject(idx);
    dropObject(idx);
    return ret;
}

function _assertBoolean(n) {
    if (typeof(n) !== 'boolean') {
        throw new Error('expected a boolean argument');
    }
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });

cachedTextDecoder.decode();

let cachegetUint8Memory0 = null;
function getUint8Memory0() {
    if (cachegetUint8Memory0 === null || cachegetUint8Memory0.buffer !== wasm.memory.buffer) {
        cachegetUint8Memory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachegetUint8Memory0;
}

function getStringFromWasm0(ptr, len) {
    return cachedTextDecoder.decode(getUint8Memory0().subarray(ptr, ptr + len));
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

function _assertNum(n) {
    if (typeof(n) !== 'number') throw new Error('expected a number argument');
}

let cachegetFloat64Memory0 = null;
function getFloat64Memory0() {
    if (cachegetFloat64Memory0 === null || cachegetFloat64Memory0.buffer !== wasm.memory.buffer) {
        cachegetFloat64Memory0 = new Float64Array(wasm.memory.buffer);
    }
    return cachegetFloat64Memory0;
}

let cachegetInt32Memory0 = null;
function getInt32Memory0() {
    if (cachegetInt32Memory0 === null || cachegetInt32Memory0.buffer !== wasm.memory.buffer) {
        cachegetInt32Memory0 = new Int32Array(wasm.memory.buffer);
    }
    return cachegetInt32Memory0;
}

let WASM_VECTOR_LEN = 0;

let cachegetNodeBufferMemory0 = null;
function getNodeBufferMemory0() {
    if (cachegetNodeBufferMemory0 === null || cachegetNodeBufferMemory0.buffer !== wasm.memory.buffer) {
        cachegetNodeBufferMemory0 = Buffer.from(wasm.memory.buffer);
    }
    return cachegetNodeBufferMemory0;
}

function passStringToWasm0(arg, malloc) {

    if (typeof(arg) !== 'string') throw new Error('expected a string argument');

    const len = Buffer.byteLength(arg);
    const ptr = malloc(len);
    getNodeBufferMemory0().write(arg, ptr, len);
    WASM_VECTOR_LEN = len;
    return ptr;
}

function debugString(val) {
    // primitive types
    const type = typeof val;
    if (type == 'number' || type == 'boolean' || val == null) {
        return  `${val}`;
    }
    if (type == 'string') {
        return `"${val}"`;
    }
    if (type == 'symbol') {
        const description = val.description;
        if (description == null) {
            return 'Symbol';
        } else {
            return `Symbol(${description})`;
        }
    }
    if (type == 'function') {
        const name = val.name;
        if (typeof name == 'string' && name.length > 0) {
            return `Function(${name})`;
        } else {
            return 'Function';
        }
    }
    // objects
    if (Array.isArray(val)) {
        const length = val.length;
        let debug = '[';
        if (length > 0) {
            debug += debugString(val[0]);
        }
        for(let i = 1; i < length; i++) {
            debug += ', ' + debugString(val[i]);
        }
        debug += ']';
        return debug;
    }
    // Test for built-in
    const builtInMatches = /\[object ([^\]]+)\]/.exec(toString.call(val));
    let className;
    if (builtInMatches.length > 1) {
        className = builtInMatches[1];
    } else {
        // Failed to match the standard '[object ClassName]'
        return toString.call(val);
    }
    if (className == 'Object') {
        // we're a user defined class or Object
        // JSON.stringify avoids problems with cycles, and is generally much
        // easier than looping through ownProperties of `val`.
        try {
            return 'Object(' + JSON.stringify(val) + ')';
        } catch (_) {
            return 'Object';
        }
    }
    // errors
    if (val instanceof Error) {
        return `${val.name}: ${val.message}\n${val.stack}`;
    }
    // TODO we could test for more things here, like `Set`s and `Map`s.
    return className;
}

function logError(e) {
    let error = (function () {
        try {
            return e instanceof Error ? `${e.message}\n\nStack:\n${e.stack}` : e.toString();
        } catch(_) {
            return "<failed to stringify thrown value>";
        }
    }());
    console.error("wasm-bindgen: imported JS function that was not marked as `catch` threw an error:", error);
    throw e;
}
/**
* @param {any} wasm_bytes
* @returns {any}
*/
module.exports.inject_contract = function(wasm_bytes) {
    var ret = wasm.inject_contract(addHeapObject(wasm_bytes));
    return takeObject(ret);
};

/**
* @param {any} mem
*/
module.exports.test_memory = function(mem) {
    wasm.test_memory(addHeapObject(mem));
};

const u32CvtShim = new Uint32Array(2);

const uint64CvtShim = new BigUint64Array(u32CvtShim.buffer);

function getArrayU8FromWasm0(ptr, len) {
    return getUint8Memory0().subarray(ptr / 1, ptr / 1 + len);
}

function handleError(e) {
    wasm.__wbindgen_exn_store(addHeapObject(e));
}
/**
*/
class VM {

    static __wrap(ptr) {
        const obj = Object.create(VM.prototype);
        obj.ptr = ptr;

        return obj;
    }

    free() {
        const ptr = this.ptr;
        this.ptr = 0;

        wasm.__wbg_vm_free(ptr);
    }
    /**
    * @param {any} context
    * @param {any} mem
    */
    constructor(context, mem) {
        var ret = wasm.vm_new(addHeapObject(context), addHeapObject(mem));
        return VM.__wrap(ret);
    }
    /**
    */
    reset() {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        wasm.vm_reset(this.ptr);
    }
    /**
    * @param {any} context
    */
    set_context(context) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        wasm.vm_set_context(this.ptr, addHeapObject(context));
    }
    /**
    * @param {any} s
    */
    set_current_account_id(s) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        wasm.vm_set_current_account_id(this.ptr, addHeapObject(s));
    }
    /**
    * @param {any} s
    */
    set_input(s) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        wasm.vm_set_input(this.ptr, addHeapObject(s));
    }
    /**
    * @param {any} s
    */
    set_signer_account_id(s) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        wasm.vm_set_signer_account_id(this.ptr, addHeapObject(s));
    }
    /**
    * The public key that was used to sign the original transaction that led to
    * this execution.
    * @param {any} s
    */
    set_signer_account_pk(s) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        wasm.vm_set_signer_account_pk(this.ptr, addHeapObject(s));
    }
    /**
    * @param {any} s
    */
    set_predecessor_account_id(s) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        wasm.vm_set_predecessor_account_id(this.ptr, addHeapObject(s));
    }
    /**
    * @param {BigInt} block_height
    */
    set_block_index(block_height) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        uint64CvtShim[0] = block_height;
        const low0 = u32CvtShim[0];
        const high0 = u32CvtShim[1];
        wasm.vm_set_block_index(this.ptr, low0, high0);
    }
    /**
    * @param {BigInt} stmp
    */
    set_block_timestamp(stmp) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        uint64CvtShim[0] = stmp;
        const low0 = u32CvtShim[0];
        const high0 = u32CvtShim[1];
        wasm.vm_set_block_timestamp(this.ptr, low0, high0);
    }
    /**
    * @param {any} u_128
    */
    set_account_balance(u_128) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        wasm.vm_set_account_balance(this.ptr, addHeapObject(u_128));
    }
    /**
    * @param {any} u_128
    */
    set_account_locked_balance(u_128) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        wasm.vm_set_account_locked_balance(this.ptr, addHeapObject(u_128));
    }
    /**
    * @param {BigInt} amt
    */
    set_storage_usage(amt) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        uint64CvtShim[0] = amt;
        const low0 = u32CvtShim[0];
        const high0 = u32CvtShim[1];
        wasm.vm_set_storage_usage(this.ptr, low0, high0);
    }
    /**
    * @param {any} u_128
    */
    set_attached_deposit(u_128) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        wasm.vm_set_attached_deposit(this.ptr, addHeapObject(u_128));
    }
    /**
    * @param {BigInt} _u64
    */
    set_prepaid_gas(_u64) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        uint64CvtShim[0] = _u64;
        const low0 = u32CvtShim[0];
        const high0 = u32CvtShim[1];
        wasm.vm_set_prepaid_gas(this.ptr, low0, high0);
    }
    /**
    * @param {any} s
    */
    set_random_seed(s) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        wasm.vm_set_random_seed(this.ptr, addHeapObject(s));
    }
    /**
    * @param {boolean} b
    */
    set_is_view(b) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        _assertBoolean(b);
        wasm.vm_set_is_view(this.ptr, b);
    }
    /**
    * @param {any} arr
    */
    set_output_data_receivers(arr) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        wasm.vm_set_output_data_receivers(this.ptr, addHeapObject(arr));
    }
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
    read_register(register_id, ptr) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        uint64CvtShim[0] = register_id;
        const low0 = u32CvtShim[0];
        const high0 = u32CvtShim[1];
        uint64CvtShim[0] = ptr;
        const low1 = u32CvtShim[0];
        const high1 = u32CvtShim[1];
        wasm.vm_read_register(this.ptr, low0, high0, low1, high1);
    }
    /**
    * @param {BigInt} register_id
    * @returns {BigInt}
    */
    register_len(register_id) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        uint64CvtShim[0] = register_id;
        const low0 = u32CvtShim[0];
        const high0 = u32CvtShim[1];
        wasm.vm_register_len(8, this.ptr, low0, high0);
        var r0 = getInt32Memory0()[8 / 4 + 0];
        var r1 = getInt32Memory0()[8 / 4 + 1];
        u32CvtShim[0] = r0;
        u32CvtShim[1] = r1;
        const n1 = uint64CvtShim[0];
        return n1;
    }
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
    current_account_id(register_id) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        uint64CvtShim[0] = register_id;
        const low0 = u32CvtShim[0];
        const high0 = u32CvtShim[1];
        wasm.vm_current_account_id(this.ptr, low0, high0);
    }
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
    signer_account_id(register_id) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        uint64CvtShim[0] = register_id;
        const low0 = u32CvtShim[0];
        const high0 = u32CvtShim[1];
        wasm.vm_signer_account_id(this.ptr, low0, high0);
    }
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
    signer_account_pk(register_id) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        uint64CvtShim[0] = register_id;
        const low0 = u32CvtShim[0];
        const high0 = u32CvtShim[1];
        wasm.vm_signer_account_pk(this.ptr, low0, high0);
    }
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
    predecessor_account_id(register_id) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        uint64CvtShim[0] = register_id;
        const low0 = u32CvtShim[0];
        const high0 = u32CvtShim[1];
        wasm.vm_predecessor_account_id(this.ptr, low0, high0);
    }
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
    input(register_id) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        uint64CvtShim[0] = register_id;
        const low0 = u32CvtShim[0];
        const high0 = u32CvtShim[1];
        wasm.vm_input(this.ptr, low0, high0);
    }
    /**
    * Returns the current block height.
    *
    * # Cost
    *
    * `base`
    * TODO #1903 rename to `block_height`
    * @returns {BigInt}
    */
    block_index() {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        wasm.vm_block_index(8, this.ptr);
        var r0 = getInt32Memory0()[8 / 4 + 0];
        var r1 = getInt32Memory0()[8 / 4 + 1];
        u32CvtShim[0] = r0;
        u32CvtShim[1] = r1;
        const n0 = uint64CvtShim[0];
        return n0;
    }
    /**
    * Returns the current block timestamp.
    *
    * # Cost
    *
    * `base`
    * @returns {BigInt}
    */
    block_timestamp() {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        wasm.vm_block_timestamp(8, this.ptr);
        var r0 = getInt32Memory0()[8 / 4 + 0];
        var r1 = getInt32Memory0()[8 / 4 + 1];
        u32CvtShim[0] = r0;
        u32CvtShim[1] = r1;
        const n0 = uint64CvtShim[0];
        return n0;
    }
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
    storage_usage() {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        wasm.vm_storage_usage(8, this.ptr);
        var r0 = getInt32Memory0()[8 / 4 + 0];
        var r1 = getInt32Memory0()[8 / 4 + 1];
        u32CvtShim[0] = r0;
        u32CvtShim[1] = r1;
        const n0 = uint64CvtShim[0];
        return n0;
    }
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
    account_balance(balance_ptr) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        uint64CvtShim[0] = balance_ptr;
        const low0 = u32CvtShim[0];
        const high0 = u32CvtShim[1];
        wasm.vm_account_balance(this.ptr, low0, high0);
    }
    /**
    * The current amount of tokens locked due to staking.
    *
    * # Cost
    *
    * `base + memory_write_base + memory_write_size * 16`
    * @param {BigInt} balance_ptr
    */
    account_locked_balance(balance_ptr) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        uint64CvtShim[0] = balance_ptr;
        const low0 = u32CvtShim[0];
        const high0 = u32CvtShim[1];
        wasm.vm_account_locked_balance(this.ptr, low0, high0);
    }
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
    attached_deposit(balance_ptr) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        uint64CvtShim[0] = balance_ptr;
        const low0 = u32CvtShim[0];
        const high0 = u32CvtShim[1];
        wasm.vm_attached_deposit(this.ptr, low0, high0);
    }
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
    prepaid_gas() {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        wasm.vm_prepaid_gas(8, this.ptr);
        var r0 = getInt32Memory0()[8 / 4 + 0];
        var r1 = getInt32Memory0()[8 / 4 + 1];
        u32CvtShim[0] = r0;
        u32CvtShim[1] = r1;
        const n0 = uint64CvtShim[0];
        return n0;
    }
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
    used_gas() {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        wasm.vm_used_gas(8, this.ptr);
        var r0 = getInt32Memory0()[8 / 4 + 0];
        var r1 = getInt32Memory0()[8 / 4 + 1];
        u32CvtShim[0] = r0;
        u32CvtShim[1] = r1;
        const n0 = uint64CvtShim[0];
        return n0;
    }
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
    random_seed(register_id) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        uint64CvtShim[0] = register_id;
        const low0 = u32CvtShim[0];
        const high0 = u32CvtShim[1];
        wasm.vm_random_seed(this.ptr, low0, high0);
    }
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
    sha256(value_len, value_ptr, register_id) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        uint64CvtShim[0] = value_len;
        const low0 = u32CvtShim[0];
        const high0 = u32CvtShim[1];
        uint64CvtShim[0] = value_ptr;
        const low1 = u32CvtShim[0];
        const high1 = u32CvtShim[1];
        uint64CvtShim[0] = register_id;
        const low2 = u32CvtShim[0];
        const high2 = u32CvtShim[1];
        wasm.vm_sha256(this.ptr, low0, high0, low1, high1, low2, high2);
    }
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
    keccak256(value_len, value_ptr, register_id) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        uint64CvtShim[0] = value_len;
        const low0 = u32CvtShim[0];
        const high0 = u32CvtShim[1];
        uint64CvtShim[0] = value_ptr;
        const low1 = u32CvtShim[0];
        const high1 = u32CvtShim[1];
        uint64CvtShim[0] = register_id;
        const low2 = u32CvtShim[0];
        const high2 = u32CvtShim[1];
        wasm.vm_keccak256(this.ptr, low0, high0, low1, high1, low2, high2);
    }
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
    keccak512(value_len, value_ptr, register_id) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        uint64CvtShim[0] = value_len;
        const low0 = u32CvtShim[0];
        const high0 = u32CvtShim[1];
        uint64CvtShim[0] = value_ptr;
        const low1 = u32CvtShim[0];
        const high1 = u32CvtShim[1];
        uint64CvtShim[0] = register_id;
        const low2 = u32CvtShim[0];
        const high2 = u32CvtShim[1];
        wasm.vm_keccak512(this.ptr, low0, high0, low1, high1, low2, high2);
    }
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
    gas(gas_amount) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        _assertNum(gas_amount);
        wasm.vm_gas(this.ptr, gas_amount);
    }
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
    promise_create(account_id_len, account_id_ptr, method_name_len, method_name_ptr, arguments_len, arguments_ptr, amount_ptr, gas) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        uint64CvtShim[0] = account_id_len;
        const low0 = u32CvtShim[0];
        const high0 = u32CvtShim[1];
        uint64CvtShim[0] = account_id_ptr;
        const low1 = u32CvtShim[0];
        const high1 = u32CvtShim[1];
        uint64CvtShim[0] = method_name_len;
        const low2 = u32CvtShim[0];
        const high2 = u32CvtShim[1];
        uint64CvtShim[0] = method_name_ptr;
        const low3 = u32CvtShim[0];
        const high3 = u32CvtShim[1];
        uint64CvtShim[0] = arguments_len;
        const low4 = u32CvtShim[0];
        const high4 = u32CvtShim[1];
        uint64CvtShim[0] = arguments_ptr;
        const low5 = u32CvtShim[0];
        const high5 = u32CvtShim[1];
        uint64CvtShim[0] = amount_ptr;
        const low6 = u32CvtShim[0];
        const high6 = u32CvtShim[1];
        uint64CvtShim[0] = gas;
        const low7 = u32CvtShim[0];
        const high7 = u32CvtShim[1];
        wasm.vm_promise_create(8, this.ptr, low0, high0, low1, high1, low2, high2, low3, high3, low4, high4, low5, high5, low6, high6, low7, high7);
        var r0 = getInt32Memory0()[8 / 4 + 0];
        var r1 = getInt32Memory0()[8 / 4 + 1];
        u32CvtShim[0] = r0;
        u32CvtShim[1] = r1;
        const n8 = uint64CvtShim[0];
        return n8;
    }
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
    promise_then(promise_idx, account_id_len, account_id_ptr, method_name_len, method_name_ptr, arguments_len, arguments_ptr, amount_ptr, gas) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        uint64CvtShim[0] = promise_idx;
        const low0 = u32CvtShim[0];
        const high0 = u32CvtShim[1];
        uint64CvtShim[0] = account_id_len;
        const low1 = u32CvtShim[0];
        const high1 = u32CvtShim[1];
        uint64CvtShim[0] = account_id_ptr;
        const low2 = u32CvtShim[0];
        const high2 = u32CvtShim[1];
        uint64CvtShim[0] = method_name_len;
        const low3 = u32CvtShim[0];
        const high3 = u32CvtShim[1];
        uint64CvtShim[0] = method_name_ptr;
        const low4 = u32CvtShim[0];
        const high4 = u32CvtShim[1];
        uint64CvtShim[0] = arguments_len;
        const low5 = u32CvtShim[0];
        const high5 = u32CvtShim[1];
        uint64CvtShim[0] = arguments_ptr;
        const low6 = u32CvtShim[0];
        const high6 = u32CvtShim[1];
        uint64CvtShim[0] = amount_ptr;
        const low7 = u32CvtShim[0];
        const high7 = u32CvtShim[1];
        uint64CvtShim[0] = gas;
        const low8 = u32CvtShim[0];
        const high8 = u32CvtShim[1];
        wasm.vm_promise_then(8, this.ptr, low0, high0, low1, high1, low2, high2, low3, high3, low4, high4, low5, high5, low6, high6, low7, high7, low8, high8);
        var r0 = getInt32Memory0()[8 / 4 + 0];
        var r1 = getInt32Memory0()[8 / 4 + 1];
        u32CvtShim[0] = r0;
        u32CvtShim[1] = r1;
        const n9 = uint64CvtShim[0];
        return n9;
    }
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
    promise_and(promise_idx_ptr, promise_idx_count) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        uint64CvtShim[0] = promise_idx_ptr;
        const low0 = u32CvtShim[0];
        const high0 = u32CvtShim[1];
        uint64CvtShim[0] = promise_idx_count;
        const low1 = u32CvtShim[0];
        const high1 = u32CvtShim[1];
        wasm.vm_promise_and(8, this.ptr, low0, high0, low1, high1);
        var r0 = getInt32Memory0()[8 / 4 + 0];
        var r1 = getInt32Memory0()[8 / 4 + 1];
        u32CvtShim[0] = r0;
        u32CvtShim[1] = r1;
        const n2 = uint64CvtShim[0];
        return n2;
    }
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
    promise_batch_create(account_id_len, account_id_ptr) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        uint64CvtShim[0] = account_id_len;
        const low0 = u32CvtShim[0];
        const high0 = u32CvtShim[1];
        uint64CvtShim[0] = account_id_ptr;
        const low1 = u32CvtShim[0];
        const high1 = u32CvtShim[1];
        wasm.vm_promise_batch_create(8, this.ptr, low0, high0, low1, high1);
        var r0 = getInt32Memory0()[8 / 4 + 0];
        var r1 = getInt32Memory0()[8 / 4 + 1];
        u32CvtShim[0] = r0;
        u32CvtShim[1] = r1;
        const n2 = uint64CvtShim[0];
        return n2;
    }
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
    promise_batch_then(promise_idx, account_id_len, account_id_ptr) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        uint64CvtShim[0] = promise_idx;
        const low0 = u32CvtShim[0];
        const high0 = u32CvtShim[1];
        uint64CvtShim[0] = account_id_len;
        const low1 = u32CvtShim[0];
        const high1 = u32CvtShim[1];
        uint64CvtShim[0] = account_id_ptr;
        const low2 = u32CvtShim[0];
        const high2 = u32CvtShim[1];
        wasm.vm_promise_batch_then(8, this.ptr, low0, high0, low1, high1, low2, high2);
        var r0 = getInt32Memory0()[8 / 4 + 0];
        var r1 = getInt32Memory0()[8 / 4 + 1];
        u32CvtShim[0] = r0;
        u32CvtShim[1] = r1;
        const n3 = uint64CvtShim[0];
        return n3;
    }
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
    promise_batch_action_create_account(promise_idx) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        uint64CvtShim[0] = promise_idx;
        const low0 = u32CvtShim[0];
        const high0 = u32CvtShim[1];
        wasm.vm_promise_batch_action_create_account(this.ptr, low0, high0);
    }
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
    promise_batch_action_deploy_contract(promise_idx, code_len, code_ptr) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        uint64CvtShim[0] = promise_idx;
        const low0 = u32CvtShim[0];
        const high0 = u32CvtShim[1];
        uint64CvtShim[0] = code_len;
        const low1 = u32CvtShim[0];
        const high1 = u32CvtShim[1];
        uint64CvtShim[0] = code_ptr;
        const low2 = u32CvtShim[0];
        const high2 = u32CvtShim[1];
        wasm.vm_promise_batch_action_deploy_contract(this.ptr, low0, high0, low1, high1, low2, high2);
    }
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
    promise_batch_action_function_call(promise_idx, method_name_len, method_name_ptr, arguments_len, arguments_ptr, amount_ptr, gas) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        uint64CvtShim[0] = promise_idx;
        const low0 = u32CvtShim[0];
        const high0 = u32CvtShim[1];
        uint64CvtShim[0] = method_name_len;
        const low1 = u32CvtShim[0];
        const high1 = u32CvtShim[1];
        uint64CvtShim[0] = method_name_ptr;
        const low2 = u32CvtShim[0];
        const high2 = u32CvtShim[1];
        uint64CvtShim[0] = arguments_len;
        const low3 = u32CvtShim[0];
        const high3 = u32CvtShim[1];
        uint64CvtShim[0] = arguments_ptr;
        const low4 = u32CvtShim[0];
        const high4 = u32CvtShim[1];
        uint64CvtShim[0] = amount_ptr;
        const low5 = u32CvtShim[0];
        const high5 = u32CvtShim[1];
        uint64CvtShim[0] = gas;
        const low6 = u32CvtShim[0];
        const high6 = u32CvtShim[1];
        wasm.vm_promise_batch_action_function_call(this.ptr, low0, high0, low1, high1, low2, high2, low3, high3, low4, high4, low5, high5, low6, high6);
    }
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
    promise_batch_action_transfer(promise_idx, amount_ptr) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        uint64CvtShim[0] = promise_idx;
        const low0 = u32CvtShim[0];
        const high0 = u32CvtShim[1];
        uint64CvtShim[0] = amount_ptr;
        const low1 = u32CvtShim[0];
        const high1 = u32CvtShim[1];
        wasm.vm_promise_batch_action_transfer(this.ptr, low0, high0, low1, high1);
    }
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
    promise_batch_action_stake(promise_idx, amount_ptr, public_key_len, public_key_ptr) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        uint64CvtShim[0] = promise_idx;
        const low0 = u32CvtShim[0];
        const high0 = u32CvtShim[1];
        uint64CvtShim[0] = amount_ptr;
        const low1 = u32CvtShim[0];
        const high1 = u32CvtShim[1];
        uint64CvtShim[0] = public_key_len;
        const low2 = u32CvtShim[0];
        const high2 = u32CvtShim[1];
        uint64CvtShim[0] = public_key_ptr;
        const low3 = u32CvtShim[0];
        const high3 = u32CvtShim[1];
        wasm.vm_promise_batch_action_stake(this.ptr, low0, high0, low1, high1, low2, high2, low3, high3);
    }
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
    promise_batch_action_add_key_with_full_access(promise_idx, public_key_len, public_key_ptr, nonce) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        uint64CvtShim[0] = promise_idx;
        const low0 = u32CvtShim[0];
        const high0 = u32CvtShim[1];
        uint64CvtShim[0] = public_key_len;
        const low1 = u32CvtShim[0];
        const high1 = u32CvtShim[1];
        uint64CvtShim[0] = public_key_ptr;
        const low2 = u32CvtShim[0];
        const high2 = u32CvtShim[1];
        uint64CvtShim[0] = nonce;
        const low3 = u32CvtShim[0];
        const high3 = u32CvtShim[1];
        wasm.vm_promise_batch_action_add_key_with_full_access(this.ptr, low0, high0, low1, high1, low2, high2, low3, high3);
    }
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
    promise_batch_action_add_key_with_function_call(promise_idx, public_key_len, public_key_ptr, nonce, allowance_ptr, receiver_id_len, receiver_id_ptr, method_names_len, method_names_ptr) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        uint64CvtShim[0] = promise_idx;
        const low0 = u32CvtShim[0];
        const high0 = u32CvtShim[1];
        uint64CvtShim[0] = public_key_len;
        const low1 = u32CvtShim[0];
        const high1 = u32CvtShim[1];
        uint64CvtShim[0] = public_key_ptr;
        const low2 = u32CvtShim[0];
        const high2 = u32CvtShim[1];
        uint64CvtShim[0] = nonce;
        const low3 = u32CvtShim[0];
        const high3 = u32CvtShim[1];
        uint64CvtShim[0] = allowance_ptr;
        const low4 = u32CvtShim[0];
        const high4 = u32CvtShim[1];
        uint64CvtShim[0] = receiver_id_len;
        const low5 = u32CvtShim[0];
        const high5 = u32CvtShim[1];
        uint64CvtShim[0] = receiver_id_ptr;
        const low6 = u32CvtShim[0];
        const high6 = u32CvtShim[1];
        uint64CvtShim[0] = method_names_len;
        const low7 = u32CvtShim[0];
        const high7 = u32CvtShim[1];
        uint64CvtShim[0] = method_names_ptr;
        const low8 = u32CvtShim[0];
        const high8 = u32CvtShim[1];
        wasm.vm_promise_batch_action_add_key_with_function_call(this.ptr, low0, high0, low1, high1, low2, high2, low3, high3, low4, high4, low5, high5, low6, high6, low7, high7, low8, high8);
    }
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
    promise_batch_action_delete_key(promise_idx, public_key_len, public_key_ptr) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        uint64CvtShim[0] = promise_idx;
        const low0 = u32CvtShim[0];
        const high0 = u32CvtShim[1];
        uint64CvtShim[0] = public_key_len;
        const low1 = u32CvtShim[0];
        const high1 = u32CvtShim[1];
        uint64CvtShim[0] = public_key_ptr;
        const low2 = u32CvtShim[0];
        const high2 = u32CvtShim[1];
        wasm.vm_promise_batch_action_delete_key(this.ptr, low0, high0, low1, high1, low2, high2);
    }
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
    promise_batch_action_delete_account(promise_idx, beneficiary_id_len, beneficiary_id_ptr) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        uint64CvtShim[0] = promise_idx;
        const low0 = u32CvtShim[0];
        const high0 = u32CvtShim[1];
        uint64CvtShim[0] = beneficiary_id_len;
        const low1 = u32CvtShim[0];
        const high1 = u32CvtShim[1];
        uint64CvtShim[0] = beneficiary_id_ptr;
        const low2 = u32CvtShim[0];
        const high2 = u32CvtShim[1];
        wasm.vm_promise_batch_action_delete_account(this.ptr, low0, high0, low1, high1, low2, high2);
    }
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
    promise_results_count() {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        wasm.vm_promise_results_count(8, this.ptr);
        var r0 = getInt32Memory0()[8 / 4 + 0];
        var r1 = getInt32Memory0()[8 / 4 + 1];
        u32CvtShim[0] = r0;
        u32CvtShim[1] = r1;
        const n0 = uint64CvtShim[0];
        return n0;
    }
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
    promise_result(result_idx, register_id) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        uint64CvtShim[0] = result_idx;
        const low0 = u32CvtShim[0];
        const high0 = u32CvtShim[1];
        uint64CvtShim[0] = register_id;
        const low1 = u32CvtShim[0];
        const high1 = u32CvtShim[1];
        wasm.vm_promise_result(8, this.ptr, low0, high0, low1, high1);
        var r0 = getInt32Memory0()[8 / 4 + 0];
        var r1 = getInt32Memory0()[8 / 4 + 1];
        u32CvtShim[0] = r0;
        u32CvtShim[1] = r1;
        const n2 = uint64CvtShim[0];
        return n2;
    }
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
    promise_return(promise_idx) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        uint64CvtShim[0] = promise_idx;
        const low0 = u32CvtShim[0];
        const high0 = u32CvtShim[1];
        wasm.vm_promise_return(this.ptr, low0, high0);
    }
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
    value_return(value_len, value_ptr) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        uint64CvtShim[0] = value_len;
        const low0 = u32CvtShim[0];
        const high0 = u32CvtShim[1];
        uint64CvtShim[0] = value_ptr;
        const low1 = u32CvtShim[0];
        const high1 = u32CvtShim[1];
        wasm.vm_value_return(this.ptr, low0, high0, low1, high1);
    }
    /**
    * Terminates the execution of the program with panic `GuestPanic`.
    *
    * # Cost
    *
    * `base`
    */
    panic() {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        wasm.vm_panic(this.ptr);
    }
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
    panic_utf8(len, ptr) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        uint64CvtShim[0] = len;
        const low0 = u32CvtShim[0];
        const high0 = u32CvtShim[1];
        uint64CvtShim[0] = ptr;
        const low1 = u32CvtShim[0];
        const high1 = u32CvtShim[1];
        wasm.vm_panic_utf8(this.ptr, low0, high0, low1, high1);
    }
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
    log_utf8(len, ptr) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        uint64CvtShim[0] = len;
        const low0 = u32CvtShim[0];
        const high0 = u32CvtShim[1];
        uint64CvtShim[0] = ptr;
        const low1 = u32CvtShim[0];
        const high1 = u32CvtShim[1];
        wasm.vm_log_utf8(this.ptr, low0, high0, low1, high1);
    }
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
    log_utf16(len, ptr) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        uint64CvtShim[0] = len;
        const low0 = u32CvtShim[0];
        const high0 = u32CvtShim[1];
        uint64CvtShim[0] = ptr;
        const low1 = u32CvtShim[0];
        const high1 = u32CvtShim[1];
        wasm.vm_log_utf16(this.ptr, low0, high0, low1, high1);
    }
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
    abort(msg_ptr, filename_ptr, line, col) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        _assertNum(msg_ptr);
        _assertNum(filename_ptr);
        _assertNum(line);
        _assertNum(col);
        wasm.vm_abort(this.ptr, msg_ptr, filename_ptr, line, col);
    }
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
    storage_write(key_len, key_ptr, value_len, value_ptr, register_id) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        uint64CvtShim[0] = key_len;
        const low0 = u32CvtShim[0];
        const high0 = u32CvtShim[1];
        uint64CvtShim[0] = key_ptr;
        const low1 = u32CvtShim[0];
        const high1 = u32CvtShim[1];
        uint64CvtShim[0] = value_len;
        const low2 = u32CvtShim[0];
        const high2 = u32CvtShim[1];
        uint64CvtShim[0] = value_ptr;
        const low3 = u32CvtShim[0];
        const high3 = u32CvtShim[1];
        uint64CvtShim[0] = register_id;
        const low4 = u32CvtShim[0];
        const high4 = u32CvtShim[1];
        wasm.vm_storage_write(8, this.ptr, low0, high0, low1, high1, low2, high2, low3, high3, low4, high4);
        var r0 = getInt32Memory0()[8 / 4 + 0];
        var r1 = getInt32Memory0()[8 / 4 + 1];
        u32CvtShim[0] = r0;
        u32CvtShim[1] = r1;
        const n5 = uint64CvtShim[0];
        return n5;
    }
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
    storage_read(key_len, key_ptr, register_id) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        uint64CvtShim[0] = key_len;
        const low0 = u32CvtShim[0];
        const high0 = u32CvtShim[1];
        uint64CvtShim[0] = key_ptr;
        const low1 = u32CvtShim[0];
        const high1 = u32CvtShim[1];
        uint64CvtShim[0] = register_id;
        const low2 = u32CvtShim[0];
        const high2 = u32CvtShim[1];
        wasm.vm_storage_read(8, this.ptr, low0, high0, low1, high1, low2, high2);
        var r0 = getInt32Memory0()[8 / 4 + 0];
        var r1 = getInt32Memory0()[8 / 4 + 1];
        u32CvtShim[0] = r0;
        u32CvtShim[1] = r1;
        const n3 = uint64CvtShim[0];
        return n3;
    }
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
    storage_remove(key_len, key_ptr, register_id) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        uint64CvtShim[0] = key_len;
        const low0 = u32CvtShim[0];
        const high0 = u32CvtShim[1];
        uint64CvtShim[0] = key_ptr;
        const low1 = u32CvtShim[0];
        const high1 = u32CvtShim[1];
        uint64CvtShim[0] = register_id;
        const low2 = u32CvtShim[0];
        const high2 = u32CvtShim[1];
        wasm.vm_storage_remove(8, this.ptr, low0, high0, low1, high1, low2, high2);
        var r0 = getInt32Memory0()[8 / 4 + 0];
        var r1 = getInt32Memory0()[8 / 4 + 1];
        u32CvtShim[0] = r0;
        u32CvtShim[1] = r1;
        const n3 = uint64CvtShim[0];
        return n3;
    }
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
    storage_has_key(key_len, key_ptr) {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        uint64CvtShim[0] = key_len;
        const low0 = u32CvtShim[0];
        const high0 = u32CvtShim[1];
        uint64CvtShim[0] = key_ptr;
        const low1 = u32CvtShim[0];
        const high1 = u32CvtShim[1];
        wasm.vm_storage_has_key(8, this.ptr, low0, high0, low1, high1);
        var r0 = getInt32Memory0()[8 / 4 + 0];
        var r1 = getInt32Memory0()[8 / 4 + 1];
        u32CvtShim[0] = r0;
        u32CvtShim[1] = r1;
        const n2 = uint64CvtShim[0];
        return n2;
    }
    /**
    * Utilities
    *Computes the outcome of execution.
    * @returns {any}
    */
    outcome() {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        var ret = wasm.vm_outcome(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {any}
    */
    created_receipts() {
        if (this.ptr == 0) throw new Error('Attempt to use a moved value');
        _assertNum(this.ptr);
        var ret = wasm.vm_created_receipts(this.ptr);
        return takeObject(ret);
    }
}
module.exports.VM = VM;

module.exports.__wbindgen_object_clone_ref = function(arg0) {
    var ret = getObject(arg0);
    return addHeapObject(ret);
};

module.exports.__wbg_log_6ce223fdee9c34a8 = function(arg0, arg1) {
    try {
        console.log(getStringFromWasm0(arg0, arg1));
    } catch (e) {
        logError(e)
    }
};

module.exports.__wbg_fitsmemory_8e38756424432b8d = function(arg0, arg1, arg2, arg3, arg4) {
    try {
        u32CvtShim[0] = arg1;
        u32CvtShim[1] = arg2;
        const n0 = uint64CvtShim[0];
        u32CvtShim[0] = arg3;
        u32CvtShim[1] = arg4;
        const n1 = uint64CvtShim[0];
        var ret = getObject(arg0).fits_memory(n0, n1);
        _assertBoolean(ret);
        return ret;
    } catch (e) {
        logError(e)
    }
};

module.exports.__wbg_readmemory_61c77ecaade64c0d = function(arg0, arg1, arg2, arg3, arg4) {
    try {
        u32CvtShim[0] = arg1;
        u32CvtShim[1] = arg2;
        const n0 = uint64CvtShim[0];
        getObject(arg0).read_memory(n0, getArrayU8FromWasm0(arg3, arg4));
    } catch (e) {
        logError(e)
    }
};

module.exports.__wbg_readmemoryu8_b854d6f121ba3eb7 = function(arg0, arg1, arg2) {
    try {
        u32CvtShim[0] = arg1;
        u32CvtShim[1] = arg2;
        const n0 = uint64CvtShim[0];
        var ret = getObject(arg0).read_memory_u8(n0);
        _assertNum(ret);
        return ret;
    } catch (e) {
        logError(e)
    }
};

module.exports.__wbg_writememory_ac21b7d629311fbb = function(arg0, arg1, arg2, arg3, arg4) {
    try {
        u32CvtShim[0] = arg1;
        u32CvtShim[1] = arg2;
        const n0 = uint64CvtShim[0];
        getObject(arg0).write_memory(n0, getArrayU8FromWasm0(arg3, arg4));
    } catch (e) {
        logError(e)
    }
};

module.exports.__wbindgen_object_drop_ref = function(arg0) {
    takeObject(arg0);
};

module.exports.__wbg_error_4bb6c2a97407129a = function(arg0, arg1) {
    try {
        try {
            console.error(getStringFromWasm0(arg0, arg1));
        } finally {
            wasm.__wbindgen_free(arg0, arg1);
        }
    } catch (e) {
        logError(e)
    }
};

module.exports.__wbg_new_59cb74e423758ede = function() {
    try {
        var ret = new Error();
        return addHeapObject(ret);
    } catch (e) {
        logError(e)
    }
};

module.exports.__wbg_stack_558ba5917b466edd = function(arg0, arg1) {
    try {
        var ret = getObject(arg1).stack;
        var ptr0 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len0;
        getInt32Memory0()[arg0 / 4 + 0] = ptr0;
    } catch (e) {
        logError(e)
    }
};

module.exports.__wbg_new_68adb0d58759a4ed = function() {
    try {
        var ret = new Object();
        return addHeapObject(ret);
    } catch (e) {
        logError(e)
    }
};

module.exports.__wbg_set_2e79e744454afade = function(arg0, arg1, arg2) {
    try {
        getObject(arg0)[takeObject(arg1)] = takeObject(arg2);
    } catch (e) {
        logError(e)
    }
};

module.exports.__wbindgen_is_undefined = function(arg0) {
    var ret = getObject(arg0) === undefined;
    _assertBoolean(ret);
    return ret;
};

module.exports.__wbindgen_is_null = function(arg0) {
    var ret = getObject(arg0) === null;
    _assertBoolean(ret);
    return ret;
};

module.exports.__wbindgen_number_new = function(arg0) {
    var ret = arg0;
    return addHeapObject(ret);
};

module.exports.__wbindgen_string_new = function(arg0, arg1) {
    var ret = getStringFromWasm0(arg0, arg1);
    return addHeapObject(ret);
};

module.exports.__wbindgen_is_object = function(arg0) {
    const val = getObject(arg0);
    var ret = typeof(val) === 'object' && val !== null;
    _assertBoolean(ret);
    return ret;
};

module.exports.__wbg_new_ec28d6ba821801cb = function() {
    try {
        var ret = new Array();
        return addHeapObject(ret);
    } catch (e) {
        logError(e)
    }
};

module.exports.__wbg_isArray_c4a3026522b7f963 = function(arg0) {
    try {
        var ret = Array.isArray(getObject(arg0));
        _assertBoolean(ret);
        return ret;
    } catch (e) {
        logError(e)
    }
};

module.exports.__wbg_push_ffaa2df7422d3b4c = function(arg0, arg1) {
    try {
        var ret = getObject(arg0).push(getObject(arg1));
        _assertNum(ret);
        return ret;
    } catch (e) {
        logError(e)
    }
};

module.exports.__wbg_instanceof_ArrayBuffer_d851e92b8b88a310 = function(arg0) {
    try {
        var ret = getObject(arg0) instanceof ArrayBuffer;
        _assertBoolean(ret);
        return ret;
    } catch (e) {
        logError(e)
    }
};

module.exports.__wbg_values_b136c3cae7c97bf6 = function(arg0) {
    try {
        var ret = getObject(arg0).values();
        return addHeapObject(ret);
    } catch (e) {
        logError(e)
    }
};

module.exports.__wbg_new_a674acf697de3e9f = function(arg0, arg1) {
    try {
        var ret = new Error(getStringFromWasm0(arg0, arg1));
        return addHeapObject(ret);
    } catch (e) {
        logError(e)
    }
};

module.exports.__wbg_call_804d3ad7e8acd4d5 = function(arg0, arg1) {
    try {
        try {
            var ret = getObject(arg0).call(getObject(arg1));
            return addHeapObject(ret);
        } catch (e) {
            handleError(e)
        }
    } catch (e) {
        logError(e)
    }
};

module.exports.__wbg_next_c08bf57a239475c3 = function(arg0) {
    try {
        try {
            var ret = getObject(arg0).next();
            return addHeapObject(ret);
        } catch (e) {
            handleError(e)
        }
    } catch (e) {
        logError(e)
    }
};

module.exports.__wbg_next_3d521c5c088358fa = function(arg0) {
    try {
        var ret = getObject(arg0).next;
        return addHeapObject(ret);
    } catch (e) {
        logError(e)
    }
};

module.exports.__wbg_done_e784c64062606540 = function(arg0) {
    try {
        var ret = getObject(arg0).done;
        _assertBoolean(ret);
        return ret;
    } catch (e) {
        logError(e)
    }
};

module.exports.__wbg_value_11f53ed6202a1367 = function(arg0) {
    try {
        var ret = getObject(arg0).value;
        return addHeapObject(ret);
    } catch (e) {
        logError(e)
    }
};

module.exports.__wbg_isSafeInteger_357be569d88701c0 = function(arg0) {
    try {
        var ret = Number.isSafeInteger(getObject(arg0));
        _assertBoolean(ret);
        return ret;
    } catch (e) {
        logError(e)
    }
};

module.exports.__wbg_iterator_7ab2f711861ad076 = function() {
    try {
        var ret = Symbol.iterator;
        return addHeapObject(ret);
    } catch (e) {
        logError(e)
    }
};

module.exports.__wbg_instanceof_Uint8Array_4342cb3c1a0c83fe = function(arg0) {
    try {
        var ret = getObject(arg0) instanceof Uint8Array;
        _assertBoolean(ret);
        return ret;
    } catch (e) {
        logError(e)
    }
};

module.exports.__wbg_new_da17c07b1fbb4a8b = function(arg0) {
    try {
        var ret = new Uint8Array(getObject(arg0));
        return addHeapObject(ret);
    } catch (e) {
        logError(e)
    }
};

module.exports.__wbg_length_b7dc6aed3ca09be1 = function(arg0) {
    try {
        var ret = getObject(arg0).length;
        _assertNum(ret);
        return ret;
    } catch (e) {
        logError(e)
    }
};

module.exports.__wbg_byteLength_5f1abb9d0f73f112 = function(arg0) {
    try {
        var ret = getObject(arg0).byteLength;
        _assertNum(ret);
        return ret;
    } catch (e) {
        logError(e)
    }
};

module.exports.__wbg_set_4a8545284001c06f = function(arg0, arg1, arg2) {
    try {
        getObject(arg0).set(getObject(arg1), arg2 >>> 0);
    } catch (e) {
        logError(e)
    }
};

module.exports.__wbg_buffer_f897a8d316863411 = function(arg0) {
    try {
        var ret = getObject(arg0).buffer;
        return addHeapObject(ret);
    } catch (e) {
        logError(e)
    }
};

module.exports.__wbindgen_is_function = function(arg0) {
    var ret = typeof(getObject(arg0)) === 'function';
    _assertBoolean(ret);
    return ret;
};

module.exports.__wbg_get_8fd175832d82a656 = function(arg0, arg1) {
    try {
        try {
            var ret = Reflect.get(getObject(arg0), getObject(arg1));
            return addHeapObject(ret);
        } catch (e) {
            handleError(e)
        }
    } catch (e) {
        logError(e)
    }
};

module.exports.__wbindgen_number_get = function(arg0, arg1) {
    const obj = getObject(arg1);
    var ret = typeof(obj) === 'number' ? obj : undefined;
    if (!isLikeNone(ret)) {
        _assertNum(ret);
    }
    getFloat64Memory0()[arg0 / 8 + 1] = isLikeNone(ret) ? 0 : ret;
    getInt32Memory0()[arg0 / 4 + 0] = !isLikeNone(ret);
};

module.exports.__wbindgen_string_get = function(arg0, arg1) {
    const obj = getObject(arg1);
    var ret = typeof(obj) === 'string' ? obj : undefined;
    var ptr0 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len0 = WASM_VECTOR_LEN;
    getInt32Memory0()[arg0 / 4 + 1] = len0;
    getInt32Memory0()[arg0 / 4 + 0] = ptr0;
};

module.exports.__wbindgen_boolean_get = function(arg0) {
    const v = getObject(arg0);
    var ret = typeof(v) === 'boolean' ? (v ? 1 : 0) : 2;
    _assertNum(ret);
    return ret;
};

module.exports.__wbindgen_debug_string = function(arg0, arg1) {
    var ret = debugString(getObject(arg1));
    var ptr0 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len0 = WASM_VECTOR_LEN;
    getInt32Memory0()[arg0 / 4 + 1] = len0;
    getInt32Memory0()[arg0 / 4 + 0] = ptr0;
};

module.exports.__wbindgen_throw = function(arg0, arg1) {
    throw new Error(getStringFromWasm0(arg0, arg1));
};

module.exports.__wbindgen_memory = function() {
    var ret = wasm.memory;
    return addHeapObject(ret);
};

const path = require('path').join(__dirname, 'near_mock_vm_bg.wasm');
const bytes = require('fs').readFileSync(path);

const wasmModule = new WebAssembly.Module(bytes);
const wasmInstance = new WebAssembly.Instance(wasmModule, imports);
wasm = wasmInstance.exports;
module.exports.__wasm = wasm;

