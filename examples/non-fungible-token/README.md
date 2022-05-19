Non-fungible Token (NFT)
===================

Example implementation of a [non-fungible token] contract which uses [near-contract-standards].

  [non-fungible token]: https://nomicon.io/Standards/NonFungibleToken/README.html
  [near-contract-standards]: https://github.com/near/near-sdk-rs/tree/master/near-contract-standards

NOTES:
 - The maximum balance value is limited by U128 (2**128 - 1).
 - JSON calls should pass [U128](https://docs.rs/near-sdk/latest/near_sdk/json_types/struct.U128.html) or [U64](https://docs.rs/near-sdk/latest/near_sdk/json_types/struct.U64.html) as a base-10 string. E.g. "100".
 - The core NFT standard does not include escrow/approval functionality, as `nft_transfer_call` provides a superior approach. Please see the approval management standard if this is the desired approach.

## Building
To build run:
```bash
./build.sh
```

## Testing
To test run:
```bash
cargo test --workspace --package non-fungible-token -- --nocapture
```
