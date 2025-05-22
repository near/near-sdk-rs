# MPC Contract

A contract that shows you how to work with the yield promise API.

## Install `cargo-near` build tool

See [`cargo-near` installation](https://github.com/near/cargo-near#installation)

## Build with:

```bash
cargo near build non-reproducible-wasm
```

# Demo reproducible build (in docker container):

```bash
cargo near build reproducible-wasm --no-locked
```

For a non-demo reproducible build/deploy a specific Cargo.lock has to be committed to git,
which is not done for demo examples in order to optimize maintenance burden.

## Testing

To test run:
```bash
cargo test
```

## Executing on localnet

Please note you would need to have three terminals open to test this example.

### TERMINAL 1: Starting localnet

```bash
npm i -g near-sandbox
near-sandbox localnet
near-sandbox init
near-sandbox run --rpc-addr 127.0.0.1:2323
```

### TERMINAL 2: Prerequisites and deployment

```bash
# preparing network config. Skip if you already configured your account.
near config add-connection --network-name localnet --connection-name sandbox --rpc-url http://127.0.0.1:2323/ --wallet-url https://app.mynearwallet.com/ --explorer-transaction-url https://explorer.near.org/transactions/

VALIDATOR_KEY=$(cat ~/.near/validator_key.json | jq .secret_key)

# adding validator account
near account import-account using-private-key $VALIDATOR_KEY network-config sandbox

# deploying contract
cargo near deploy build-non-reproducible-wasm test.near without-init-call network-config sandbox sign-with-keychain send
```

### Actual test

#### TERMINAL 2
One account will send a request for signing: This account will hang on transaction waiting for other account to send data on-chain to resume the transaction.
```bash
near contract call-function as-transaction test.near 'sign' json-args '{ "message": "message-to-sign" }' prepaid-gas '200.0 Tgas' attached-deposit '0 NEAR' sign-as test.near network-config sandbox sign-with-keychain send
```

After this you should see that it's waiting.

#### TERMINAL 3
This terminal will send the data on-chain to resume the transaction. After this transaction is completed, the first transaction will be resumed and completed. Please note that you need to do that before timeout happens (200 blocks in this case).
```bash
# This should return a non-null value
near --quiet contract call-function as-read-only test.near get_requests json-args {} network-config sandbox now | jq first.data_id

DATA_ID=$(near --quiet contract call-function as-read-only test.near get_requests json-args {} network-config sandbox now | jq first.data_id)

near contract call-function as-transaction test.near 'sign_respond' json-args '{ "data_id": '$DATA_ID', "signature": "signature" }' prepaid-gas '200.0 Tgas' attached-deposit '0 NEAR' sign-as test.near network-config sandbox sign-with-keychain send
```

The second terminal should now be resumed and completed.
