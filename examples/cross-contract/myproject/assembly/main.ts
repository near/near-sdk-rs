//@nearfile
import { near, context, storage, logging } from "near-runtime-ts";
import { Greeter } from "./model";

// --- contract code goes below

// It's good to use common constant, but not required.
const LAST_SENDER_KEY = "last_sender";

// This is our change method. It modifies the state of the contract by
// storing the account_id of the sender under the key "last_sender" on the blockchain
export function sayHi(): void {
  // context.sender is the account_id of the user who sent this call to the contract
  // It's provided by the Blockchain runtime. For now we just store it in a local variable.
  let sender = context.sender;
  // `near` class contains some helper functions, e.g. logging.
  // Logs are not persistently stored on the blockchain, but produced by the blockchain runtime.
  // It's helpful to use logs for debugging your functions or when you need to get some info
  // from the change methods (since change methods don't return values to the front-end).
  logging.log(sender + " says \"Hello mate!\"");
  // storage is a helper class that allows contracts to modify the persistent state
  // and read from it. setString allows you to persitently store a string value for a given string key.
  // We'll store the last sender of this contract who called this method.
  storage.setString(LAST_SENDER_KEY, sender);
}

// This is our view method. It returns the last account_id of a sender who called `sayHi`.
// It reads value from the persistent store under the key "last_sender" and returns it.
export function whoSaidHi(): string | null {
  // getString returns a string value for a given string key.
  return storage.getString(LAST_SENDER_KEY);
}
