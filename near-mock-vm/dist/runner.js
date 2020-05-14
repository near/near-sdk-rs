"use strict";
/// <reference lib="dom" />
var __importStar = (this && this.__importStar) || function (mod) {
    if (mod && mod.__esModule) return mod;
    var result = {};
    if (mod != null) for (var k in mod) if (Object.hasOwnProperty.call(mod, k)) result[k] = mod[k];
    result["default"] = mod;
    return result;
};
Object.defineProperty(exports, "__esModule", { value: true });
const near_mock_vm_1 = require("../pkg/near_mock_vm");
const context_1 = require("./context");
const memory_1 = require("./memory");
const js_base64_1 = require("js-base64");
const loader = __importStar(require("@assemblyscript/loader"));
const utils = __importStar(require("./utils"));
class VMRunner {
    constructor(memory, contextPath) {
        this.wasm = null;
        this.gas = 0;
        const context = context_1.createContext(contextPath);
        this.vm = new near_mock_vm_1.VM(context, memory);
        this.memory = memory;
    }
    static create(memory, contextPath) {
        let mem = new memory_1.Memory(memory);
        return new VMRunner(mem, contextPath);
    }
    static instrumentBinary(binary) {
        return Buffer.from(near_mock_vm_1.inject_contract(binary));
    }
    readUTF8Str(ptr) {
        let arr = [];
        let mem = this.memory.memory;
        while (mem[ptr] != 0) {
            arr.push(mem[ptr]);
            ptr++;
        }
        return utils.UTF8toStr(arr);
        // return this.memory.readUTF8Str(s);
    }
    createImports() {
        let vm = this.vm;
        let self = this;
        let memory = this.memory.Memory;
        let _imports = {
            vm: {
                restoreState() {
                    vm.reset();
                },
                outcome() {
                    let outcome = vm.outcome();
                    let strArrPtr = self.wasm.newStringArray();
                    for (let str of outcome.logs) {
                        strArrPtr = self.wasm.pushString(strArrPtr, self.wasm.__allocString(str));
                    }
                    let return_data_ptr;
                    if (outcome.return_data === "None") {
                        return_data_ptr = self.wasm.NONE;
                    }
                    const balancePtr = self.wasm.__allocString(outcome.balance);
                    let outcomePtr = new self.wasm.Outcome(balancePtr, BigInt(outcome.burnt_gas), BigInt(outcome.used_gas), strArrPtr, BigInt(outcome.storage_usage), return_data_ptr);
                    return outcomePtr.valueOf();
                },
                // saveContext() {
                //   vm.save_context();
                // },
                // restoreContext() {
                //   vm.restore_context();
                // },
                setCurrent_account_id(s) {
                    vm.set_current_account_id(self.readUTF8Str(s));
                },
                setInput(s) {
                    vm.set_input(js_base64_1.Base64.encode(self.readUTF8Str(s)));
                },
                setSigner_account_id(s) {
                    vm.set_signer_account_id(self.readUTF8Str(s));
                },
                /// The public key that was used to sign the original transaction that led to
                /// this execution.
                setSigner_account_pk(s) {
                    vm.set_signer_account_pk(self.readUTF8Str(s));
                },
                setPredecessor_account_id(s) {
                    vm.set_predecessor_account_id(self.readUTF8Str(s));
                },
                setBlock_index(block_height) {
                    vm.set_block_index(block_height);
                },
                setBlock_timestamp(stmp) {
                    vm.set_block_timestamp(stmp);
                },
                setAccount_balance(lo, hi) {
                    //TODO: actually  u128
                    vm.set_account_balance(utils.createU128Str(lo, hi));
                },
                setAccount_locked_balance(lo, hi) {
                    vm.set_account_locked_balance(utils.createU128Str(lo, hi));
                },
                setStorage_usage(amt) {
                    vm.set_storage_usage(amt);
                },
                setAttached_deposit(lo, hi) {
                    vm.set_attached_deposit(utils.createU128Str(lo, hi));
                },
                setPrepaid_gas(_u64) {
                    vm.set_prepaid_gas(_u64);
                },
                setRandom_seed(s) {
                    vm.set_random_seed(self.readUTF8Str(s));
                },
                setIs_view(b) {
                    vm.set_is_view(b == 1);
                },
            },
            env: {
                memory,
                /// #################
                /// # Registers API #
                /// #################
                // write_register(data_len, data_ptr, register_id: BigInt) {
                //   return vm.write_register(data_len, data_ptr, register_id);
                // },
                read_register(register_id, ptr) {
                    return vm.read_register(register_id, ptr);
                },
                register_len(register_id) {
                    return vm.register_len(register_id);
                },
                // ###############
                // # Context API #
                // ###############
                current_account_id(register_id) {
                    return vm.current_account_id(register_id);
                },
                signer_account_id(register_id) {
                    return vm.signer_account_id(register_id);
                },
                signer_account_pk(register_id) {
                    return vm.signer_account_pk(register_id);
                },
                predecessor_account_id(register_id) {
                    return vm.predecessor_account_id(register_id);
                },
                input(register_id) {
                    return vm.input(register_id);
                },
                block_index() {
                    return vm.block_index();
                },
                block_timestamp() {
                    return vm.block_timestamp();
                },
                storage_usage() {
                    return vm.storage_usage();
                },
                // #################
                // # Economics API #
                // #################
                account_balance(balance_ptr) {
                    return vm.account_balance(balance_ptr);
                },
                attached_deposit(balance_ptr) {
                    return vm.attached_deposit(balance_ptr);
                },
                prepaid_gas() {
                    return vm.prepaid_gas();
                },
                used_gas() {
                    return vm.used_gas();
                },
                // ############
                // # Math API #
                // ############
                random_seed(register_id) {
                    return vm.random_seed(register_id);
                },
                sha256(value_len, value_ptr, register_id) {
                    return vm.sha256(value_len, value_ptr, register_id);
                },
                keccak256(value_len, value_ptr, register_id) {
                    return vm.keccak256(value_len, value_ptr, register_id);
                },
                keccak512(value_len, value_ptr, register_id) {
                    return vm.keccak512(value_len, value_ptr, register_id);
                },
                // #####################
                // # Miscellaneous API #
                // #####################
                value_return(value_len, value_ptr) {
                    return vm.value_return(value_len, value_ptr);
                },
                panic() {
                    return vm.panic();
                },
                log_utf8(len, ptr) {
                    return vm.log_utf8(len, ptr);
                },
                log_utf16(len, ptr) {
                    return vm.log_utf16(len, ptr);
                },
                // ################
                // # Promises API #
                // ################
                promise_create(account_id_len, account_id_ptr, method_name_len, method_name_ptr, arguments_len, arguments_ptr, amount_ptr, gas) {
                    return vm.promise_create(account_id_len, account_id_ptr, method_name_len, method_name_ptr, arguments_len, arguments_ptr, amount_ptr, gas);
                },
                promise_then(promise_index, account_id_len, account_id_ptr, method_name_len, method_name_ptr, arguments_len, arguments_ptr, amount_ptr, gas) {
                    return vm.promise_then(promise_index, account_id_len, account_id_ptr, method_name_len, method_name_ptr, arguments_len, arguments_ptr, amount_ptr, gas);
                },
                promise_and(promise_idx_ptr, promise_idx_count) {
                    return vm.promise_and(promise_idx_ptr, promise_idx_count);
                },
                promise_results_count() {
                    return vm.promise_results_count();
                },
                promise_result(result_idx, register_id) {
                    return vm.promise_result(result_idx, register_id);
                },
                promise_return(promise_id) {
                    return vm.promise_return(promise_id);
                },
                promise_batch_create(account_id_len, account_id_ptr) {
                    return vm.promise_batch_create(account_id_len, account_id_ptr);
                },
                promise_batch_then(promise_index, account_id_len, account_id_ptr) {
                    return vm.promise_batch_then(promise_index, account_id_len, account_id_ptr);
                },
                // #######################
                // # Promise API actions #
                // #######################
                promise_batch_action_create_account(promise_index) {
                    return vm.promise_batch_action_create_account(promise_index);
                },
                promise_batch_action_deploy_contract(promise_index, code_len, code_ptr) {
                    return vm.promise_batch_action_deploy_contract(promise_index, code_len, code_ptr);
                },
                promise_batch_action_function_call(promise_index, method_name_len, method_name_ptr, arguments_len, arguments_ptr, amount_ptr, gas) {
                    return vm.promise_batch_action_function_call(promise_index, method_name_len, method_name_ptr, arguments_len, arguments_ptr, amount_ptr, gas);
                },
                promise_batch_action_transfer(promise_index, amount_ptr) {
                    return vm.promise_batch_action_transfer(promise_index, amount_ptr);
                },
                promise_batch_action_stake(promise_index, amount_ptr, public_key_len, public_key_ptr) {
                    return vm.promise_batch_action_stake(promise_index, amount_ptr, public_key_len, public_key_ptr);
                },
                promise_batch_action_add_key_with_full_access(promise_index, public_key_len, public_key_ptr, nonce) {
                    return vm.promise_batch_action_add_key_with_full_access(promise_index, public_key_len, public_key_ptr, nonce);
                },
                promise_batch_action_add_key_with_function_call(promise_index, public_key_len, public_key_ptr, nonce, allowance_ptr, receiver_id_len, receiver_id_ptr, method_names_len, method_names_ptr) {
                    return vm.promise_batch_action_add_key_with_function_call(promise_index, public_key_len, public_key_ptr, nonce, allowance_ptr, receiver_id_len, receiver_id_ptr, method_names_len, method_names_ptr);
                },
                promise_batch_action_delete_key(promise_index, public_key_len, public_key_ptr) {
                    return vm.promise_batch_action_delete_key(promise_index, public_key_len, public_key_ptr);
                },
                promise_batch_action_delete_account(promise_index, beneficiary_id_len, beneficiary_id_ptr) {
                    return vm.promise_batch_action_delete_account(promise_index, beneficiary_id_len, beneficiary_id_ptr);
                },
                // ###############
                // # Storage API #
                // ###############
                storage_write(key_len, key_ptr, value_len, value_ptr, register_id) {
                    return vm.storage_write(key_len, key_ptr, value_len, value_ptr, register_id);
                },
                storage_read(key_len, key_ptr, register_id) {
                    return vm.storage_read(key_len, key_ptr, register_id);
                },
                storage_remove(key_len, key_ptr, register_id) {
                    return vm.storage_remove(key_len, key_ptr, register_id);
                },
                storage_has_key(key_len, key_ptr) {
                    return vm.storage_has_key(key_len, key_ptr);
                },
                // // Function for the injected gas counter. Automatically called by the gas meter.
                gas(gas_amount) {
                    self.gas += gas_amount;
                    return vm.gas(gas_amount);
                }
            }
        };
        return _imports;
    }
    run(method, input) {
        this.vm.set_input(js_base64_1.Base64.encode(input));
        this.wasm[method]();
    }
    static setup(binary, contextPath, memory) {
        const vm = VMRunner.create(memory, contextPath);
        const instrumented_bin = VMRunner.instrumentBinary(binary);
        const wasm = loader.instantiateSync(instrumented_bin, vm.createImports());
        vm.wasm = wasm;
        vm.memory = new memory_1.Memory(wasm.memory);
        return vm;
    }
    outcome() {
        return this.vm.outcome();
    }
    created_receipts() {
        return this.vm.created_receipts();
    }
    static run(binary, method, input, contextPath) {
        const runner = VMRunner.setup(binary, contextPath);
        runner.run(method, input);
        let after = runner.outcome();
        // console.log(after);
        console.log("calls to injected gas: " + runner.gas);
        console.log("Gas used after startup: " + ((after.used_gas) / (10 ** 12)));
        console.log("Outcome:");
        console.log(after);
        const receipts = runner.created_receipts();
        if (receipts.length > 0) {
            console.log("Receipts: ");
            console.log(receipts);
        }
    }
}
exports.VMRunner = VMRunner;
//# sourceMappingURL=runner.js.map