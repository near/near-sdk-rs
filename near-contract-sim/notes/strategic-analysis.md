# Strategic Analysis: Is near-contract-sim Worth It?

## The Core Question

We built a working in-process contract testing runtime. But should we continue investing in it?

**The appeal:** Not running a full node just to test contracts. 10-100x faster than near-workspaces.

**The concern:** We're reimplementing nearcore, poorly. And `near-vm-runner` isn't a great primitive.

## The Gap With nearcore

| What we have | What nearcore has | Gap |
|--------------|-------------------|-----|
| Function calls | Full action types | Transfer, Deploy, Stake, AddKey, DeleteKey, DeleteAccount |
| Basic storage | Storage staking | Balance checks, storage cost enforcement |
| Simple receipts | Full receipt model | Gas refunds, data receipts, execution order guarantees |
| ~1200 lines | ~50k+ lines | **Massive** |

**The uncomfortable truth:** To have a useful simulation, you need ~80% of nearcore's runtime logic. We have maybe 15%.

## Why near-vm-runner Is The Wrong Primitive

1. **Action log is a hack**
   - It's a `Vec<String>` that we parse with string matching
   - Not a structured API - we're reverse-engineering

2. **MockedExternal isn't designed for us**
   - It's meant for nearcore's internal tests
   - Not a public contract testing API

3. **No contract→runtime feedback loop**
   - Can't simulate "this transfer failed because insufficient balance"
   - The VM doesn't model balances, accounts, or economics

4. **Configuration is weird**
   - `RuntimeConfigStore::test()` vs `::free()` is an internal concern
   - We're exposing implementation details

## What's The Actual Value?

### Where near-contract-sim helps:
- Testing pure compute logic with cross-contract calls
- Fast iteration on callback chains (where economics don't matter)
- Mocking external dependencies
- CI speed (no 100MB sandbox binary download)

### Where it definitely doesn't help:
- Any test involving transfers, account creation, staking
- Tests where storage costs or gas economics matter
- Anything requiring nearcore-accurate behavior
- Tests that need to match mainnet/testnet behavior exactly

## Three Paths Forward

### Path 1: Kill It

Accept that `near-workspaces` is the answer for cross-contract tests. Focus on making `testing_env!` better for single-contract unit tests.

**Pros:**
- No maintenance burden
- Clear testing story: unit tests OR integration tests
- No "false confidence" from incomplete simulation

**Cons:**
- The gap between unit and integration remains
- Slow CI for cross-contract scenarios

### Path 2: Upstream It

The *right* place for this is in nearcore or a nearcore-maintained crate. The NEAR team could expose a proper `near-runtime-sim` that:
- Uses the real runtime logic (not reimplementing it)
- Exposes a clean testing API
- Is maintained alongside protocol changes

**Pros:**
- Full fidelity with nearcore
- Maintained by the right team
- Could become the official fast-test solution

**Cons:**
- Requires buy-in from NEAR team
- Bigger commitment (RFC, coordination)
- May not be a priority for them

**Action:** Could propose as an RFC or GitHub discussion.

### Path 3: Ship It As "Lite"

Keep it, but be explicit about scope:

> "This is a fast approximation for testing cross-contract call patterns. For economic or state-mutation accuracy, use workspaces."

**Pros:**
- Delivers value for the narrow use case
- Honest about limitations
- Low maintenance if scope is fixed

**Cons:**
- Risk of users expecting more than it provides
- Still need to maintain as nearcore evolves
- Action log parsing could break

## Questions To Answer Before Deciding

1. **Is there actual demand?**
   - Do real NEAR developers want this?
   - What tests do they write that are too slow with workspaces?

2. **What's the maintenance burden?**
   - How often does nearcore's action log format change?
   - Will this break every protocol upgrade?

3. **Could near-workspaces be made faster instead?**
   - In-process sandbox mode?
   - Persistent sandbox with state reset?
   - Memory-backed storage?

## Current Recommendation

**Pause and validate demand.**

Before investing more:
1. Talk to NEAR developers about their testing pain points
2. Measure actual workspaces overhead vs contract-sim
3. Explore if workspaces itself could be optimized

If there's real demand for "fast cross-contract tests that don't need full fidelity," Path 3 (ship as lite) is pragmatic.

If the goal is "testing that matches production," Path 2 (upstream) is the right long-term answer, but requires coordination.

Path 1 (kill) is honest if the answer to "is there demand?" is "not really."
