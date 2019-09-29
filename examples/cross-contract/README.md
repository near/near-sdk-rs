# Cross contract

Example of using cross-contract functions, like promises, or money transfers.

## Several contracts
This tutorial demonstrates how to deploy several contracts and test interaction between them.

First, follow the [Running Rust Contract](https://github.com/nearprotocol/near-bindgen#running-rust-contract) section
from the main documentation to start the local node but use `rm -rf testdir; ./scripts/start_unittest.py --image nearprotocol/nearcore:staging` command, create the project and deploy the `status_message` contract.

Then deploy the `cross-contract` contract:
```bash
near create_account cross_contract --masterAccount=test.near --homeDir=../nearcore/testdir
near deploy --accountId=cross_contract --homeDir=../nearcore/testdir --wasmFile=../examples/cross-contract/res/cross_contract.wasm
```

### Trying money transfer

First check the balance on both `status_message` and `cross_contract` accounts:

```bash
near state status_message
near state cross_contract
```

See that they both have `amount: '100000000'`.

Then call a function on `cross_contract` that transfers money to `status_message`:

```bash
near call cross_contract transfer_money "{\"account_id\": \"status_message\", \"amount\":1000}" --accountId=test.near --homeDir=../nearcore/testdir
```

Then check the balances again:

```bash
near state status_message
near state cross_contract
```

Observe that `cross_contract` was deducted `1000` and `status_message` has balance increased by `1000`, even though
`test.near` signed the transaction and paid for all the gas that was used.

### Trying simple cross contract call

Call `simple_call` function on `cross_contract` account:

```bash
near call cross_contract simple_call "{\"account_id\": \"status_message\", \"message\":\"bonjour\"}" --accountId=test.near --homeDir=../nearcore/testdir
```

Verify that this actually resulted in correct state change in `status_message` contract:

```bash
near call status_message get_status "{\"account_id\":\"test.near\"}" --accountId=test.near --homeDir=../nearcore/testdir
```
Observe:
```bash
Result: bonjour
```

### Trying complex cross contract call

Call `complex_call` function on `cross_contract` account:

```bash
near call cross_contract complex_call "{\"account_id\": \"status_message\", \"message\":\"halo\"}" --accountId=test.near --homeDir=../nearcore/testdir
```

observe `Result: halo`.

What did just happen?

1. `test.near` account signed a transaction that called a `complex_call` method on `cross_contract` smart contract.
2. `cross_contract` executed `complex_call` with `account_id: "status_message", message: "halo"` arguments;
    1. During the execution the promise #0 was created to call `set_status` method on `status_message` with arguments `"message": "halo"`;
    2. Then another promise #1 was scheduled to be executed right after promise #0. Promise #1 was to call `get_status` on `status_message` with arguments: `"message": "test.near""`;
    3. Then the return value of `get_status` is programmed to be the return value of `complex_call`;
3. `status_message` executed `set_status`, then `status_message` executed `get_status` and got the `"halo"` return value
which is then passed as the return value of `complex_call`.

### Trying callback with return values

Call `merge_sort` function on `cross_contract` account:

```bash
near call cross_contract merge_sort "{\"arr\": [2, 1, 0, 3]}" --accountId=test.near --homeDir=../nearcore/testdir
```

observe `Result: [ 0, 1, 2, 3 ]`
