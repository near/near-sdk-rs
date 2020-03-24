import { u128 } from "near-runtime-ts";
import { Outcome } from "./outcome";

/**
 * Methods on the current VM
 */ 
export declare namespace VM {
  // /** 
  //  * Saves the internal state of the VM. 
  //  * */ 
  // //@ts-ignore
  // @external("vm", "saveState")
  // export function saveState(): void;

  /** 
   * Restores the internal state of the VM. 
   * 
   */ 
  //@ts-ignore
  @external("vm", "restoreState")
  export function restoreState(): void;

  /**
   * Return the VM Outcome of the current running contract
   */ 
  //@ts-ignore
  @external("vm", "outcome")
  export function outcome(): Outcome;

  
}  


//@ts-ignore
@external("vm", "setCurrent_account_id")
declare function _setCurrent_account_id(id: usize): void;

//@ts-ignore
@external("vm", "setInput")
declare function _setInput(input: usize): void;

//@ts-ignore
@external("vm", "setSigner_account_id")
declare function _setSigner_account_id(s: usize): void;
/// The public key that was used to sign the original transaction that led to
/// this execution.
//@ts-ignore
@external("vm", "setSigner_account_pk")
declare function _setSigner_account_pk(s: usize): void;
//@ts-ignore
@external("vm", "setPredecessor_account_id")
declare function _setPredecessor_account_id(s: usize): void;
//@ts-ignore
@external("vm", "setRandom_seed")
declare function _setRandom_seed(s: usize): void;

//@ts-ignore
@external("vm", "setAttached_deposit")
declare function _setAttached_deposit(lo: u64, hi: u64): void;

//@ts-ignore
@external("vm", "setAccount_balance")
declare function _setAccount_balance(lo: u64, hi: u64): void;

//@ts-ignore
@external("vm", "setAccount_locked_balance")
declare function _setAccount_locked_balance(lo: u64, hi: u64): void;


// //@ts-ignore
// @external("vm", "saveContext")
// declare function _saveContext(): void;

// //@ts-ignore
// @external("vm", "restoreContext")
// declare function _restoreContext(): void;

//@ts-ignore
@external("vm", "setBlock_index")
declare function _setBlock_index(block_height: u64): void;
//@ts-ignore
@external("vm", "setBlock_timestamp")
declare function _setBlock_timestamp(stmp: u64): void;

//@ts-ignore
@external("vm", "setPrepaid_gas")
declare function _setPrepaid_gas(_u64: u64): void;

//@ts-ignore
@external("vm", "setIs_view")
declare function _setIs_view(b: bool): void;
//@ts-ignore
@external("vm", "setOutput_data_receivers")
declare function _setOutput_data_receivers(arrA: Array<string>): void;
//@ts-ignore
@external("vm", "setStorage_usage")
declare function _setStorage_usage(amt: u64): void;
/**
 * Functions to edit the current VM's context
 */
export namespace Context {

  // export function saveContext(): void {
  //   _saveContext();
  // }

  // export function restoreContext(): void {
  //   _restoreContext();
  // }

  export function setCurrent_account_id(id: string): void {
    _setCurrent_account_id(changetype<usize>(String.UTF8.encode(id)));
  }

  export function setInput(input: string): void {
    _setInput(changetype<usize>(String.UTF8.encode(input)));
  }

  export function setSigner_account_id(s: string): void {
    _setSigner_account_id(changetype<usize>(String.UTF8.encode(s)));
  }
  /// The public key that was used to sign the original transaction that led to
  /// this execution.
  export function setSigner_account_pk(s: string): void {
    _setSigner_account_pk(changetype<usize>(String.UTF8.encode(s)));
  }
  export function setPredecessor_account_id(s: string): void {
    _setPredecessor_account_id(changetype<usize>(String.UTF8.encode(s)));
  }

  export function setBlock_index(block_height: u64): void {
    _setBlock_index(block_height);
  }
  
  export function setBlock_timestamp(stmp: u64): void {
    _setBlock_timestamp(stmp);
  }

  export function setAccount_balance(_u128: u128): void {
    _setAccount_balance(_u128.lo, _u128.hi);
  }

  export function setAccount_locked_balance(_u128: u128): void {
    _setAccount_locked_balance(_u128.lo, _u128.hi);
  }

  export function setStorage_usage(amt: u64): void {
    _setStorage_usage(amt);
  }

  export function setAttached_deposit(_u128: u128): void {
    _setAttached_deposit(_u128.lo, _u128.hi);
  }

  export function setPrepaid_gas(_u64: u64): void {
    _setPrepaid_gas(_u64);
  }

  export function setRandom_seed(s: string): void {
    _setRandom_seed(changetype<usize>(String.UTF8.encode(s)));
  }

  export function setIs_view(b: bool): void {
    _setIs_view(b);
  }
  
  export function setOutput_data_receivers(arrA: Array<string>): void {
    _setOutput_data_receivers(arrA);
  }
}