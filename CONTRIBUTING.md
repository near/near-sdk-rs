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
- Code is formatted with `rustfmt` by running `cargo fmt`
- Run `clippy`
  - The exact command run by the CI is `cargo clippy --tests -- -Dclippy::all`
- Run tests with `cargo test`
- Rebuild examples with docker if any breaking changes with `./examples/build_all_docker.sh`
  - We track changes in the example wasm blobs to make sure no unwanted code bloat is added with changes
- Test all examples with `./examples/test_all.sh`
  - This must be done after the previous step
- Ensure any new functionality is adequately tested
- If any new public types or functions are added, ensure they have appropriate [rustdoc](https://doc.rust-lang.org/rustdoc/what-is-rustdoc.html) documentation
