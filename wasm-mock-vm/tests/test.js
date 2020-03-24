let rust = require("../pkg/wasm_mock_vm");
let assert = require("assert");
let utils = require('./utils');
let bs58 = require("bs58");
let bs64 = require("js-base64").Base64;
const memory = new Uint8Array(10000);

// Returns whether the memory interval is completely inside the smart contract memory.
global.fits_memory = (offset, len) => utils.toNum(offset) + utils.toNum(len) < memory.length;

// Reads the content of the given memory interval.
//
// # Panics
//
// If memory interval is outside the smart contract memory.
global.read_memory = (offset, buffer) => {
    offset = utils.toNum(offset)
    buffer.set(memory.slice(offset, offset + buffer.length), 0)
};

// Reads a single byte from the memory.
//
// # Panics
//
// If pointer is outside the smart contract memory.
global.read_memory_u8 = (offset) => memory[utils.toNum(offset)];

// Writes the buffer into the smart contract memory.
//
// # Panics
//
// If `offset + buffer.len()` is outside the smart contract memory.
global.write_memory = (offset, buffer) => {
    memory.set(buffer, utils.toNum(offset));
}

const current_account_id = "alice"; 
const signer_account_id = "bob";
const signer_account_pk = "HuxUynD5GdrcZ5MauxJuu74sGHgS6wLfCqqhQkLWK";
const predecessor_account_id = "carol";
const input_str = "{ arg1: 1 }";
const input = bs64.encode(input_str);
const block_index = 10;
const block_timestamp = 42;
const account_balance = 2;
const account_locked_balance = 1;
const storage_usage = 12;
const attached_deposit = 2;
const prepaid_gas = 10**(14);
const random_seed = "HuxUynD5GdrcZ5MauxJuu74sGHgS6wLfCqqhQkLWK";
const is_view = false;
const output_data_receivers= new Uint8Array([]);


context =  {
    /// The account id of the current contract that we are executing.
    current_account_id, // string
    /// The account id of that signed the original transaction that led to this
    /// execution.
    signer_account_id, // string
    /// The public key that was used to sign the original transaction that led to
    /// this execution.
    signer_account_pk,
    predecessor_account_id,
    input,
    block_index,
    block_timestamp,
    account_balance,
    account_locked_balance,
    storage_usage,
    attached_deposit,
    prepaid_gas,
    random_seed,
    is_view,
    output_data_receivers,
}

function readReg(id) {
    const ptr = 10;
    let len = utils.toNum(vm.register_len(BigInt(id)));
    let before = memory.slice(ptr, ptr + len);
    vm.read_register(BigInt(id), BigInt(ptr));
    const res = memory.slice(ptr, ptr + utils.toNum(len));
    memory.set(before, ptr);
    return res;
}




// let map = new Array();
// map[0] = [0, new Uint8Array([42])];
let vm = new rust.VM(context);
vm.signer_account_pk(BigInt(1));
// vm.read_register(BigInt(1), BigInt(1));
assert.equal(bs58.encode(Buffer.from(readReg(1))), signer_account_pk);
debugger;
// vm.read_register(BigInt(0), BigInt(0));
// assert(memory[0] == 42);
// memory[1] = 85;
// vm.write_register(BigInt(1), BigInt(1), BigInt(1));
// assert.equal(readReg(1)[0], 85);

vm.input(BigInt(0));
assert.equal(utils.UTF8toStr(readReg(0)), input_str);

function storage_write(_key, _value) {
    let key = utils.StrtoUTF8(_key);
    let value = utils.StrtoUTF8(_value);
    memory.set(key, 1000);
    memory.set(value, 2000);
    vm.storage_write(BigInt(key.length), BigInt(1000), BigInt(value.length), BigInt(2000), BigInt(1));
    vm.storage_read(BigInt(key.length), BigInt(1000), BigInt(1));
}

function storage_read(_key, ptr) {
    let key = utils.StrtoUTF8(_key);
    memory.set(key, ptr);
    vm.storage_read(BigInt(key.length), BigInt(ptr), BigInt(0));
    return utils.UTF8toStr(readReg(0));
}

function storage_has_key(_key) {
    let key = utils.StrtoUTF8(_key);
    const saved = memory.slice(1000, 1000 + key.length);
    memory.set(key, 1000);
    let res = vm.storage_has_key(BigInt(key.length), BigInt(1000));
    memory.set(saved, 1000);
    return res;
}



const data = "I am data";
storage_write("key", data);
assert.equal(storage_read("key", 1000), data)
// assert.deepEqual(utils.UTF8toStr(memory.slice(1000, 1000 + toNum(vm.register_len(BigInt(0))))), data);
assert(storage_has_key("key"));

var errored = false
// vm.read_register(BigInt(10), BigInt(0));
storage_write("key1", data);

let key = utils.StrtoUTF8("key");
// const saved = memory.slice(1000, 1000 + key.length);
memory.set(key, 2000);

// let iterid = vm.storage_iter_prefix(BigInt(key.length), BigInt(2000));
// console.log(iterid);
// vm.storage_iter_next(iterid, BigInt(2), BigInt(3));
// console.log(utils.UTF8toStr(readReg(2)),utils.UTF8toStr(readReg(3)));
// vm.storage_iter_next(iterid, BigInt(2), BigInt(3));
// console.log(utils.UTF8toStr(readReg(2)),utils.UTF8toStr(readReg(3)));

// storage_write("aa", "bar1");
// storage_write("aaa", "bar2");
// storage_write("ab", "bar2");
// storage_write("abb", "bar3");

// key = utils.StrtoUTF8("key");
// // const saved = memory.slice(1000, 1000 + key.length);
// memory.set(key, 2000);

key2 = utils.StrtoUTF8("key9");
// const saved = memory.slice(1000, 1000 + key.length);
memory.set(key2, 3000);

// iterid = vm.storage_iter_range(BigInt(key.length), BigInt(2000), BigInt(key2.length), BigInt(3000));
// console.log(iterid);
// while (vm.storage_iter_next(iterid, BigInt(2), BigInt(3)) > 0) {
//     console.log(utils.UTF8toStr(readReg(2)),utils.UTF8toStr(readReg(3)));
// }
str = utils.StrtoUTF8("hello dolly!");
memory.set(str, 4000);
vm.log_utf8(BigInt(str.length), BigInt(4000));


console.log(vm.outcome());
// vm.read_register(BigInt(0), BigInt(0));
// assert(memory[0] == 84);

// vm.signer_account_pk(BigInt(1));

// assert.equal(vm.storage_usage(), storage_usage);
// assert.equal(vm.used_gas(), used_gas);
// assert.equal(vm.)

vm.set_current_account_id("Bobby");
vm.current_account_id(BigInt(0));
console.log(utils.UTF8toStr(readReg(0)));
assert.equal(storage_read("key", 1000), data)

// rust.pass_context(context);
// rust.set_context(new VMContext());
// vm.abort();
console.log("PASSED!");