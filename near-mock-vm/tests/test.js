let runner = require("..").VMRunner;
let assert = require("assert");
let utils = require('./utils');


let vm = runner.create();
const memory = vm.memory.memory;
vm = vm.vm;
vm.signer_account_pk(BigInt(1));
ptr = 0;
let len = utils.toNum(vm.register_len(BigInt(1)));
vm.read_register(BigInt(1), BigInt(ptr));

function readReg(id) {
    const ptr = 10;
    let len = utils.toNum(vm.register_len(BigInt(id)));
    let before = memory.slice(ptr, ptr + len);
    vm.read_register(BigInt(id), BigInt(ptr));
    const res = memory.slice(ptr, ptr + utils.toNum(len));
    memory.set(before, ptr);
    return res;
}
// vm.read_register(BigInt(1), BigInt(1));
// assert.equal(bs58.encode(Buffer.from(readReg(1))), signer_account_pk);

debugger;
// vm.read_register(BigInt(0), BigInt(0));
// assert(memory[0] == 42);
// memory[1] = 85;
// vm.write_register(BigInt(1), BigInt(1), BigInt(1));
// assert.equal(readReg(1)[0], 85);

// vm.input(BigInt(0));
// assert.equal(utils.UTF8toStr(readReg(0)), input_str);

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