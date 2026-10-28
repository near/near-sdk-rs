# Security Policy

This policy covers the crates published from this repository: `near-sdk`, `near-sdk-macros`, `near-sys`, `near-contract-standards` and the other workspace crates on crates.io.

## Reporting a vulnerability

**Please do NOT open a GitHub issue to report a security vulnerability.** Submit a report via HackenProof, with a proof of concept and a suggested fix if you have one:

- NEAR Smart Contract program: https://hackenproof.com/programs/near-smart-contracts-1
- NEAR Protocol program: https://hackenproof.com/near/near-protocol

If you cannot submit through HackenProof, tell us why at security@near.org or security@nearone.org, and we will help you. Please don't contact project contributors directly.

Security topics that are not vulnerabilities, such as ideas for hardening, are welcome as GitHub issues.

## Supported versions

| Version | Security fixes |
| ------- | -------------- |
| Latest 5.x minor release | Yes |
| Older 5.x minor releases | Upgrade to the latest 5.x |
| 4.x and earlier | No. Advisories still list these versions when they are affected |

## How we disclose vulnerabilities

Every fix to a security-relevant bug in a published crate gets a public advisory. A fix does not ship silently, even when the bug was found internally.

For each advisory we publish:

- a [GitHub Security Advisory](https://github.com/near/near-sdk-rs/security/advisories) on this repository, with a CVE requested through GitHub;
- an entry in the [RustSec Advisory Database](https://rustsec.org/), so that `cargo audit` and `cargo deny` report it;
- the affected and patched version ranges, including old major versions that we no longer patch;
- whether the fix is backported, and if not, the upgrade path.

Smart contracts are often deployed once and never rebuilt, so dependency tooling alone won't reach most affected users. When a bug can be exploited in contracts that are already deployed:

1. We fix it in a private fork or a draft advisory.
2. We look for affected contracts on mainnet and contact their owners privately, through the NEAR security team, so that they can upgrade, withdraw funds or pause first.
3. We publish the advisory once the embargo ends. The embargo is 30 days after the fix is ready, or immediately if the bug is already being exploited.

Code derived from these crates is patched in the same release cycle: the near-sdk-js contract standards, the near-examples contracts and the tutorial templates. Each advisory says which of them are affected.

Maintainers: the steps are in [`.github/SECURITY_ADVISORY_PROCESS.md`](.github/SECURITY_ADVISORY_PROCESS.md).
