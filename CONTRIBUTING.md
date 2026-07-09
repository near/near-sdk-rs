# Contributing to near-sdk-rs

Thank you for your interest in contributing to NEAR's Rust SDK! We appreciate any type of contribution.

If you have any questions about contributing, or about the project in general, please ask in our [rust-sdk Discord channel](https://discord.gg/cKRZCqD2b2).

## Code of Conduct

We have an open and welcoming environment, please review our [code of conduct](CODE_OF_CONDUCT.md).

## Development

### Commits

Please use descriptive PR titles. We loosely follow the [conventional commits](https://www.conventionalcommits.org/en/v1.0.0/) style, but this is not a requirement to follow exactly. PRs will be addressed more quickly if it is clear what the intention is.

### Workspace crate policy

This workspace is a set of independently-published crates, not a place to file every new idea as its own crate. A few rules keep it maintainable:

- **Crates are compilation boundaries, not topics.** A new workspace crate needs a hard technical justification: a proc-macro crate, an FFI/`sys` layer, or an external consumer that demonstrably cannot use a feature-gated module instead. By default, new NEP types and utilities go into an existing crate as a feature-gated module.

- **Features are additive-only.** Enabling a feature may add public API, but must never change or remove it — concretely, no `cfg(not(feature = "..."))` on a public item. This isn't hypothetical: [#1585](https://github.com/near/near-sdk-rs/issues/1585) was a real downstream feature-unification break caused by exactly this pattern on a trait supertrait. The mechanical part is enforced by CI (the `feature-additivity` job) across `near-sdk-core`, `near-sdk-env`, `near-crypto-hash`, `near-global-contracts`, and `near-digest`.

- **The leaf-crate dependency graph stays one-directional and acyclic.** Low-level crates (`near-sys`, `near-sdk-env`, `near-crypto-hash`) must not depend on higher-level ones, and there must be no cycles between leaves.

- **Leaf crates published for off-chain use** — `near-crypto-hash`, `near-global-contracts`, `near-digest` — keep a frozen, non-optional dependency list and a documented MSRV floor (currently 1.88). Raising either requires explicit maintainer sign-off, called out in the PR description.

### Before opening a PR

Ensure the following are satisfied before opening a PR:

- The `git-hooks.sh` script has been run to install the git hooks.
- Code is formatted with `rustfmt` by running `cargo fmt`
- Before running the tests, ensure that all example `.wasm` files are built by executing [./examples/build_all.sh](./examples/build_all.sh)
- Run all tests and linters with [./run-tests.sh](./run-tests.sh)
- Ensure any new functionality is adequately tested
- If any new public types or functions are added, ensure they have appropriate [rustdoc](https://doc.rust-lang.org/rustdoc/what-is-rustdoc.html) documentation
