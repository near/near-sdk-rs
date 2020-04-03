/// <reference lib="dom" />

import { VM, inject_contract } from "../pkg"
import { createContext } from './context';
import { Memory } from "./memory";
import { Base64 } from 'js-base64';
import * as loader from "@assemblyscript/loader";
import * as utils from "./utils";

export class NearVM {
  vm: VM;
  wasm: any | null = null;
  memory: Memory;
  gas: number = 0;

  constructor(memory: Memory, contextPath?: string) {
    const context = createContext(contextPath);
    this.vm = new VM(context, memory);
    this.memory = memory;
  }

  static create(memory?: WebAssembly.Memory, contextPath?: string): NearVM {
    let mem: Memory = new Memory(memory);
    return new NearVM(mem, contextPath);
  }

  static instrumentBinary(binary: Uint8Array): Uint8Array {
    return Buffer.from(inject_contract(binary));
  }

  readUTF8Str(ptr: number): string {
    let arr = [];
    let mem = this.memory.memory;
    while (mem[ptr] != 0) {
      arr.push(mem[ptr]);
      ptr++;
    }
    return utils.UTF8toStr(arr);
    // return this.memory.readUTF8Str(s);
  }

  createImports(): any {
    let vm = this.vm;
    let self = this;
    let memory = this.memory.Memory;
    let _imports =  {
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
            let outcomePtr = new self.wasm.Outcome(BigInt(outcome.balance1),
                                                BigInt(outcome.balance2),
                                                BigInt(outcome.burnt_gas),
                                                BigInt(outcome.used_gas),
                                                strArrPtr,
                                                BigInt(outcome.storage_usage),
                                                return_data_ptr)
            return outcomePtr.valueOf();
          },
          // saveContext() {
          //   vm.save_context();
          // },
          // restoreContext() {
          //   vm.restore_context();
          // },
          setCurrent_account_id(s: number) {
            vm.set_current_account_id(self.readUTF8Str(s));
          },
          setInput(s: number) {
            vm.set_input(Base64.encode(self.readUTF8Str(s)));
          },
          setSigner_account_id(s: number) {
            vm.set_signer_account_id(self.readUTF8Str(s));
          }, // string
          /// The public key that was used to sign the original transaction that led to
          /// this execution.
          setSigner_account_pk(s: number) {
            vm.set_signer_account_pk(self.readUTF8Str(s));
          }, // string base58
          setPredecessor_account_id(s: number) {
            vm.set_predecessor_account_id(self.readUTF8Str(s));
          }, // string
          setBlock_index(block_height: BigInt) {
            vm.set_block_index(block_height);
          }, // u128
          setBlock_timestamp(stmp: BigInt) {
            vm.set_block_timestamp(stmp);
          },
          setAccount_balance(lo: BigInt, hi: BigInt) {
            //TODO: actually  u128
            vm.set_account_balance(utils.createU128Str(lo, hi));
          },
          setAccount_locked_balance(lo: BigInt, hi: BigInt) {
            vm.set_account_locked_balance(utils.createU128Str(lo, hi));
          },
          setStorage_usage(amt: BigInt) {
            vm.set_storage_usage(amt);
          },
          setAttached_deposit(lo: BigInt, hi: BigInt) {
            vm.set_attached_deposit(utils.createU128Str(lo, hi));
          },
          setPrepaid_gas(_u64: BigInt) {
            vm.set_prepaid_gas(_u64);
          },
          setRandom_seed(s: number) {
            vm.set_random_seed(self.readUTF8Str(s));
          },
          setIs_view(b: number) {
            vm.set_is_view(b==1);
          },
          // setOutput_data_receivers(arr) {
          //   vm.set_output_data_receivers(arr);
          // },
        },
        env: {
          memory,
          /// #################
          /// # Registers API #
          /// #################
          // write_register(data_len, data_ptr, register_id: BigInt) {
          //   return vm.write_register(data_len, data_ptr, register_id);
          // },
          read_register(register_id: BigInt, ptr: BigInt) {
              return vm.read_register(register_id, ptr);
          },
          register_len(register_id: BigInt) {
              return vm.register_len(register_id);
          },
          // ###############
          // # Context API #
          // ###############
          current_account_id(register_id: BigInt) {
            return vm.current_account_id(register_id);
          },
          signer_account_id(register_id: BigInt) {
            return vm.signer_account_id(register_id);
          },
          signer_account_pk(register_id: BigInt) {
            return vm.signer_account_pk(register_id);
          },
          predecessor_account_id(register_id: BigInt) {
            return vm.predecessor_account_id(register_id);
          },
          input(register_id: BigInt) {
            return vm.input(register_id);
          },
          block_index(): BigInt {
            return vm.block_index();
          },
          storage_usage(): BigInt {
            return vm.storage_usage();
          },
  
          // #################
          // # Economics API #
          // #################
          account_balance(balance_ptr: BigInt) {
            return vm.account_balance(balance_ptr);
          },
          attached_deposit(balance_ptr: BigInt) {
            return vm.attached_deposit(balance_ptr);
          },
          prepaid_gas(): BigInt {
            return vm.prepaid_gas();
          },
          used_gas(): BigInt {
            return vm.used_gas();
          },
  
          // ############
          // # Math API #
          // ############
          random_seed(register_id: BigInt) {
            return vm.random_seed(register_id);
          },
          sha256(value_len: BigInt, value_ptr: BigInt, register_id: BigInt) {
            return vm.sha256(value_len, value_ptr, register_id);
          },
          keccak256(value_len: BigInt, value_ptr: BigInt, register_id: BigInt) {
            return vm.keccak256(value_len, value_ptr, register_id);
          },
          keccak512(value_len: BigInt, value_ptr: BigInt, register_id: BigInt) {
            return vm.keccak512(value_len, value_ptr, register_id);
          },
  
          // #####################
          // # Miscellaneous API #
          // #####################
          value_return(value_len: BigInt, value_ptr: BigInt) {
            return vm.value_return(value_len, value_ptr);
          },
          panic() {
            return vm.panic();
          },
          log_utf8(len: BigInt, ptr: BigInt) {
            return vm.log_utf8(len, ptr);
          },
          log_utf16(len: BigInt, ptr: BigInt) {
            return vm.log_utf16(len, ptr);
          },
  
          // ################
          // # Promises API #
          // ################
          promise_create(account_id_len: BigInt,account_id_ptr: BigInt,method_name_len: BigInt,method_name_ptr: BigInt,arguments_len: BigInt,arguments_ptr: BigInt,amount_ptr: BigInt,gas: BigInt) {
            return vm.promise_create(account_id_len,account_id_ptr,method_name_len,method_name_ptr,arguments_len, arguments_ptr, amount_ptr, gas);
          },
          promise_then(promise_index: BigInt,account_id_len: BigInt,account_id_ptr: BigInt,method_name_len: BigInt,method_name_ptr: BigInt,arguments_len: BigInt,arguments_ptr: BigInt,amount_ptr: BigInt,gas: BigInt) {
            return vm.promise_then( promise_index, account_id_len, account_id_ptr, method_name_len, method_name_ptr, arguments_len, arguments_ptr, amount_ptr, gas);
          },
          promise_and(promise_idx_ptr: BigInt,promise_idx_count: BigInt) {
            return vm.promise_and(promise_idx_ptr, promise_idx_count);
          },
          promise_results_count() {
            return vm.promise_results_count();
          },
          promise_result(result_idx: BigInt, register_id: BigInt) {
            return vm.promise_result(result_idx, register_id);
          },
          promise_return(promise_id: BigInt) {
            return vm.promise_return(promise_id);
          },
          promise_batch_create(account_id_len: BigInt,account_id_ptr: BigInt) {
            return vm.promise_batch_create(account_id_len, account_id_ptr);
          },
          promise_batch_then(promise_index: BigInt, account_id_len: BigInt,account_id_ptr: BigInt) {
            return vm.promise_batch_then(promise_index, account_id_len, account_id_ptr);
          },
  
          // #######################
          // # Promise API actions #
          // #######################
          promise_batch_action_create_account(promise_index: BigInt) {
            return vm.promise_batch_action_create_account(promise_index);
          },
          promise_batch_action_deploy_contract(promise_index: BigInt, code_len: BigInt,code_ptr: BigInt) {
            return vm.promise_batch_action_deploy_contract(promise_index, code_len, code_ptr);
          },
          promise_batch_action_function_call(promise_index: BigInt, method_name_len: BigInt, method_name_ptr: BigInt, arguments_len: BigInt, arguments_ptr: BigInt, amount_ptr: BigInt, gas: BigInt) {
            return vm.promise_batch_action_function_call(promise_index, method_name_len, method_name_ptr, arguments_len, arguments_ptr, amount_ptr, gas);
          },
          promise_batch_action_transfer(promise_index: BigInt,amount_ptr: BigInt) {
            return vm.promise_batch_action_transfer(promise_index, amount_ptr);
          },
          promise_batch_action_stake(promise_index: BigInt, amount_ptr: BigInt, public_key_len: BigInt, public_key_ptr: BigInt) {
            return vm.promise_batch_action_stake(promise_index, amount_ptr, public_key_len, public_key_ptr);
          },
          promise_batch_action_add_key_with_full_access(promise_index: BigInt, public_key_len: BigInt, public_key_ptr:BigInt, nonce: BigInt) {
            return vm.promise_batch_action_add_key_with_full_access(promise_index, public_key_len, public_key_ptr, nonce);
          },
          promise_batch_action_add_key_with_function_call(promise_index: BigInt, public_key_len: BigInt, public_key_ptr: BigInt, nonce: BigInt, allowance_ptr: BigInt, receiver_id_len: BigInt, receiver_id_ptr: BigInt, method_names_len: BigInt, method_names_ptr: BigInt) {
            return vm.promise_batch_action_add_key_with_function_call(promise_index, public_key_len, public_key_ptr, nonce, allowance_ptr, receiver_id_len, receiver_id_ptr, method_names_len, method_names_ptr);
          },
          promise_batch_action_delete_key(promise_index: BigInt, public_key_len: BigInt, public_key_ptr: BigInt) {
            return vm.promise_batch_action_delete_key(promise_index, public_key_len, public_key_ptr);
          },
          promise_batch_action_delete_account(promise_index: BigInt, beneficiary_id_len: BigInt, beneficiary_id_ptr: BigInt) {
            return vm.promise_batch_action_delete_account(promise_index, beneficiary_id_len, beneficiary_id_ptr);
          },
  
          // ###############
          // # Storage API #
          // ###############
          storage_write(key_len: BigInt, key_ptr: BigInt, value_len: BigInt, value_ptr: BigInt, register_id: BigInt) {
            return vm.storage_write(key_len, key_ptr, value_len, value_ptr, register_id);
          },
          storage_read(key_len: BigInt, key_ptr: BigInt, register_id: BigInt) {
            return vm.storage_read(key_len, key_ptr, register_id);
          },
          storage_remove(key_len: BigInt, key_ptr: BigInt, register_id: BigInt) {
            return vm.storage_remove(key_len, key_ptr, register_id);
          },
          storage_has_key(key_len: BigInt, key_ptr: BigInt) {
            return vm.storage_has_key(key_len, key_ptr);
          },
          // // Function for the injected gas counter. Automatically called by the gas meter.
          gas(gas_amount: number) {
            self.gas += gas_amount;
            return vm.gas(gas_amount);
          }
      }
    };
    return _imports;
  }
  
  static run(binary: Uint8Array, method: string, input: string): any {
    const vm = NearVM.create();
    const instrumented_bin = NearVM.instrumentBinary(binary);
    const wasm = loader.instantiateSync(instrumented_bin, vm.createImports());
    vm.wasm = wasm;
    vm.memory.Memory = wasm.memory!;
    vm.vm.set_input(Base64.encode(input));
    //@ts-ignore
    vm.wasm[method]();
    let after = vm.vm.outcome();
    // console.log(after);
    console.log("calls to injected gas: " + vm.gas);
    console.log("Gas used after startup: " + ((after.used_gas)/(10**12)) )
    console.log("Outcome:");
    console.log(after);
    const receipts = vm.vm.created_receipts();
    if (receipts.length > 0) {
      console.log("Receipts: ");
      console.log(receipts);
    }
  }
}