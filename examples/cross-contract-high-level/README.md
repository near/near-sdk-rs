# Cross contract

Example of using cross-contract functions, like promises, or money transfers.

## Several contracts
We will use three tabs (or separate windows) in Terminal for this example:
- **example tab** - this example project (likely `…/example-rust-high-level-cross-contract`)
- **nearcore tab** - nearcore running an instance
- **app tab** - small JavaScript app generated with `create-near-app`

Let's start the NEAR localnet so we can run a smart contract on it.

* Make sure you have [Docker](https://www.docker.com/) installed
* Clone the [nearprotocol/nearcore](https://github.com/nearprotocol/nearcore) repository

To start your local node, go to the **nearcore tab** and run the following commands:
```bash
rm -rf ~/.near
./scripts/start_localnet.py
```

This will pull the docker image and start a single local node.
When prompted, enter the account name `test_near` to be associated with that account.
Just to be clear, this account does not live on testnet but on the localnet. It is not possible to, for example, log into this account with the NEAR Wallet after this example.

Then execute the following to follow the block production logs:
```bash
docker logs --follow nearcore
```

In a new tab, create a project:
```bash
npx create-near-app --vanilla myproject
cd myproject
yarn
```

This tab is the **app tab**. In `src/config.json` change the `CONTRACT_NAME` to be `test_near`:

Replace:
```javascript
const CONTRACT_NAME = process.env.CONTRACT_NAME || 'near-blank-project';
```

with:
```javascript
const CONTRACT_NAME = process.env.CONTRACT_NAME || 'test_near';
```

In the same file, modify `nodeUrl` to point to your local node:
```javascript
case 'development':
    return {
        networkId: 'default',
        nodeUrl: 'http://localhost:3030', // ⟵ change this line
        contractName: CONTRACT_NAME,
        walletUrl: 'https://wallet.nearprotocol.com',
    };
```

Then copy the key that the node generated upon starting in your local project to use for transaction signing.
(The `~/.near` folder was populated when we started the localnet in the **nearcore tab**)
```bash
mkdir ./neardev/default
cp ~/.near/validator_key.json ./neardev/default/test_near.json
```

Create the `cross_contract` account from `test_near` with an initial balance:
```bash
near create_account cross_contract --masterAccount test_near  --initialBalance 10000000
```

Now we'll deploy the `cross-contract` contract.
The next step is going to involve taking a path from the **example tab**, so please modify with the correct path on your machine.
The best way to find the full path of the **example tab** is to enter the following command in the project root of this example:
```bash
pwd
```

```bash
near deploy --accountId cross_contract --wasmFile /YOUR_PATH_HERE/cross-contract-high-level/res/cross_contract_high_level.wasm
```

### Deploying another contract
Let's deploy another contract using `cross-contract`, factory-style.
```bash
near call cross_contract deploy_status_message '{"account_id": "status_message", "amount":1000000000000000}' --accountId test_near
```

### Trying money transfer

First check the balance on both `status_message` and `cross_contract` accounts:
```bash
near state cross_contract
near state status_message
```

See that cross_contract has approximately `9,999,999` and status_message has `0.000000001` tokens.

Then call a function on `cross_contract` that transfers money to `status_message`:
```bash
near call cross_contract transfer_money '{"account_id": "status_message", "amount":1000000000000000}' --accountId test_near
```

Then check the balances again:
```bash
near state cross_contract
near state status_message
```

Observe that `status_message` has `0.000000002` tokens, even though
`test_near` signed the transaction and paid for all the gas that was used.

### Trying simple cross contract call

Call `simple_call` function on `cross_contract` account:
```bash
near call cross_contract simple_call '{"account_id": "status_message", "message":"bonjour"}' --accountId test_near --gas 10000000000000000000
```

Verify that this actually resulted in correct state change in `status_message` contract:
```bash
near call status_message get_status '{"account_id":"test_near"}' --accountId test_near --gas 10000000000000000000
```

Observe:
```bash
bonjour
```

### Trying complex cross contract call

Call `complex_call` function on `cross_contract` account:
```bash
near call cross_contract complex_call '{"account_id": "status_message", "message":"halo"}' --accountId test_near --gas 10000000000000000000
```

Observe:
```bash
halo
```

What just happened?

1. `test_near` account signed a transaction that called a `complex_call` method on `cross_contract` smart contract.
2. `cross_contract` executed `complex_call` with `account_id: "status_message", message: "halo"` arguments;
    1. During the execution the promise #0 was created to call `set_status` method on `status_message` with arguments `"message": "halo"`;
    2. Then another promise #1 was scheduled to be executed right after promise #0. Promise #1 was to call `get_status` on `status_message` with arguments: `"message": "test_near""`;
    3. Then the return value of `get_status` is programmed to be the return value of `complex_call`;
3. `status_message` executed `set_status`, then `status_message` executed `get_status` and got the `"halo"` return value
which is then passed as the return value of `complex_call`.

### Trying callback with return values

Call `merge_sort` function on `cross_contract` account:
```bash
near call cross_contract merge_sort '{"arr": [2, 1, 0, 3]}' --accountId test_near --gas 10000000000000000000
```

observe the logs:
```
[cross_contract]: Received [2] and [1]
[cross_contract]: Merged [1, 2]
[cross_contract]: Received [0] and [3]
[cross_contract]: Merged [0, 3]
[cross_contract]: Received [1, 2] and [0, 3]
[cross_contract]: Merged [0, 1, 2, 3]
```

and the output
```
'\u0004\u0000\u0000\u0000\u0000\u0001\u0002\u0003'
```
The reason why output is a binary is because we used [Borsh](http://borsh.io) binary serialization format to communicate
between the contracts instead of JSON. Borsh is faster and costs less gas. In this simple example you can even read
the format, here `\u0004\u0000\u0000\u0000` stands for `4u32` encoded using little-endian encoding which corresponds to the
length of the array, `\u0000\u0001\u0002\u0003` are the elements of the array. Since the array has type `Vec<u8>` each
element is exactly one byte.

If you don't want to use it you can remove `#[serializer(borsh)]` annotation everywhere from the code and the contract will fallback to JSON.

Finally, you may stop the running docker container(s) by running this command in the **nearcore tab**:
```bash
./scripts/stop.py
```
