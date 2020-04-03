import { Base64 } from "js-base64";
import * as path from "path";


export interface VMContext {
  current_account_id: string;
  signer_account_id: string;
  signer_account_pk: string
  predecessor_account_id: string
  //Base64 encoded
  // input_str: string;
  input: string // = bs64.encode(input_str);
  block_index: number
  block_timestamp: number;
  epoch_height: number;
  account_balance: number;
  account_locked_balance: number;
  storage_usage: number;
  attached_deposit: number;
  prepaid_gas: number;
  random_seed: string 
  is_view: boolean;
  output_data_receivers: Uint8Array
}

function findContext(_path: string = ""): VMContext | null {
  let paths = [
                _path,
                path.join(process.cwd(),
                "assembly", "__tests__"),
                process.cwd(), 
                __dirname
              ]
              .map(p => {
                if (!p.endsWith("context.json")) {
                  return path.join(p, "context.json")
                }
                return p;
              });
  let _paths = paths.filter(p => {
      try {
        require.resolve(p);
      } catch {
          return false;
      }
      return true
  });
  let context: VMContext = _paths.length > 0 ? require(_paths[0]) : null;
  if (context != null) {
    console.log("found path: " + _paths[0]);
    context.input = Base64.encode(context.input);
  }
  return context;
  
}



const current_account_id = "alice"; 
const signer_account_id = "bob";
const signer_account_pk = "HuxUynD5GdrcZ5MauxJuu74sGHgS6wLfCqqhQkLWK";
const predecessor_account_id = "carol";
const input_str = "{\"a\":21,\"b\":21}";
const block_index = 10;
const block_timestamp = 42;
const epoch_height = 20;
const account_balance = 2;
const account_locked_balance = 1;
const storage_usage = 12;
const attached_deposit = 2;
const prepaid_gas = 10**(14);
const random_seed = "HuxUynD5GdrcZ5MauxJuu74sGHgS6wLfCqqhQkLWK";
const is_view = false;
const output_data_receivers= new Uint8Array([]);

export function createDefault(): VMContext {
  const _default = {
    /// The account id of the current contract that we are executing.
    current_account_id, // string
    /// The account id of that signed the original transaction that led to this
    /// execution.
    signer_account_id, // string
    /// The public key that was used to sign the original transaction that led to
    /// this execution.
    signer_account_pk, // string base58
    predecessor_account_id, // string
    input: Base64.encode(input_str), // JSON string
    block_index, // u128
    block_timestamp,
    epoch_height,
    account_balance,
    account_locked_balance,
    storage_usage,
    attached_deposit,
    prepaid_gas,
    random_seed,
    is_view,
    output_data_receivers,
  }
  return _default
}


export function createContext(_path?: string): VMContext {
  let context =  findContext(_path) || createDefault();
  context.input = Base64.encode(context.input);
  return context;
}