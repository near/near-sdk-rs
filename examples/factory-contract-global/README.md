# Factory Contract with Global Contracts Example

This example demonstrates how to use NEAR's global contract functionality to deploy and use global smart contracts.

Global contracts allow sharing contract code globally across the NEAR network, reducing deployment costs and enabling efficient code reuse.

## Key Features

- Deploy a global contract using `deploy_global_contract()`
- Use an existing global contract by hash with `use_global_contract()`
- Use an existing global contract by deployer account with `use_global_contract_by_account_id()`
- Integration tests using near-kit and near-sandbox

## Install `cargo-near` build tool

See [`cargo-near` installation](https://github.com/near/cargo-near#installation)

## Build with:

```bash
cargo near build
```

## Run Tests:

### Unit Tests
```bash
cargo test --lib
```

### Integration Tests
```bash
cargo test --test integration
```

## Create testnet dev-account:

```bash
cargo near create-dev-account
```

## Deploy to dev-account:

```bash
cargo near deploy
```

## How Global Contracts Work

1. **Deploy Global Contract**: A contract deploys bytecode as a global contract, making it available network-wide
2. **Use by Hash**: Other contracts can reference the global contract by its code hash
3. **Use by Account**: Contracts can reference a global contract by the account that deployed it

This reduces storage costs and enables code sharing across the ecosystem.

## Use Cases from NEP-591

- **Multisig Contracts**: Deploy once, use for many wallets without paying 3N each time
- **Smart Contract Wallets**: Efficient user onboarding with chain signatures
- **Business Onboarding**: Companies can deploy user accounts cost-effectively
- **DeFi Templates**: Share common contract patterns across protocols
