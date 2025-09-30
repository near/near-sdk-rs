# Contributing to near-sdk-rs

Thank you for your interest in contributing to NEAR's Rust SDK! We appreciate any type of contribution.

If you have any questions about contributing, or about the project in general, please ask in our [rust-sdk Discord channel](https://discord.gg/cKRZCqD2b2).

## Code of Conduct

We have an open and welcoming environment, please review our [code of conduct](CODE_OF_CONDUCT.md).

## Development

### Commits

Please use descriptive PR titles. We loosely follow the [conventional commits](https://www.conventionalcommits.org/en/v1.0.0/) style, but this is not a requirement to follow exactly. PRs will be addressed more quickly if it is clear what the intention is.

### Before opening a PR

Ensure the following are satisfied before opening a PR:

- The `git-hooks.sh` script has been run to install the git hooks.
- Code is formatted with `rustfmt` by running `cargo fmt`
- Before running the tests, ensure that all example `.wasm` files are built by executing [./examples/build_all.sh](./examples/build_all.sh)
- Run all tests and linters with [./run-tests.sh](./run-tests.sh)
- Ensure any new functionality is adequately tested
- If any new public types or functions are added, ensure they have appropriate [rustdoc](https://doc.rust-lang.org/rustdoc/what-is-rustdoc.html) documentation
