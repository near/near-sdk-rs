# Security advisory process (maintainers)

This is how [SECURITY.md](../SECURITY.md) is carried out. Work in private until the advisory is published.

## 1. Triage

A change is security-relevant if it fixes any of the following in a published crate:

- loss or theft of funds, or wrong balance or storage accounting;
- broken access control or authorization checks;
- state corruption, such as storage key collisions;
- a panic or gas exhaustion that an attacker can trigger to block funds;
- soundness bugs in `unsafe` code or the host-function bindings.

Ask that question about every bug fix, not only about reported vulnerabilities. If the answer is yes, or you aren't sure:

- add the `security` label to the issue or PR;
- don't describe the impact in a public issue or in the PR body until the advisory is published;
- tell the NEAR security team at security@nearone.org.

A PR with the `security` label doesn't merge until the advisory decision below is recorded in the draft advisory.

## 2. Draft the advisory

1. Open a draft at <https://github.com/near/near-sdk-rs/security/advisories/new>, or with `gh api -X POST repos/near/near-sdk-rs/security-advisories`.
2. Fill in:
   - **Ecosystem:** `Rust` (crates.io). Name the exact crate, for example `near-contract-standards`.
   - **Affected versions:** a range such as `>= 3.2.0, < 5.0.0`. Check every published version, including old majors (`git log -L` on the affected function across the tags). Don't stop at the supported ones.
   - **Patched versions:** the first release that contains the fix.
   - **Severity:** use CVSS, and score the impact on contracts that use the crate.
   - **Description:**
     - what goes wrong, and under which conditions;
     - who is affected: contracts built with the affected versions that use the affected feature;
     - the fix and the upgrade path;
     - code derived from the crate that is affected too, such as near-sdk-js standards, near-examples and templates.
   - **Credits:** only with the reporter's consent.
3. Use the draft's temporary private fork for the fix, if the fix itself would reveal the bug.

## 3. Find and notify deployed contracts

If contracts that are already deployed can be exploited:

- List mainnet contracts built with the affected versions. Signals include version strings in the wasm, exported method sets and known template code hashes.
- Keep the ones with funds at risk.
- Send the list to the NEAR security team, who contact the owners privately.
- Record in the draft which contracts can't be fixed, for example those with no access keys.

## 4. Publish

When the embargo ends, as defined in SECURITY.md:

1. Release the patched version, plus backports if any, through the usual release process.
2. Request a CVE from the draft advisory, then publish the advisory.
3. File the RustSec entry: open a PR to [rustsec/advisory-db](https://github.com/rustsec/advisory-db) adding `crates/<crate>/RUSTSEC-0000-0000.md`. Use the GHSA id as an alias (`aliases = ["GHSA-..."]`) and copy the same `patched` and `unaffected` ranges. The RustSec maintainers assign the final id.
4. Patch and announce the derived code in the same cycle, filing a GHSA on the downstream repository where it applies.
5. Add a line to the crate's CHANGELOG under the release, linking the advisory.
