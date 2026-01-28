# near-contract-sim: Design Notes

**Status: Exploratory / On Hold**

This directory contains informal notes documenting the design, implementation, and strategic analysis of `near-contract-sim` - a lightweight multi-contract testing runtime for NEAR smart contracts.

## Contents

- **[what-we-built.md](./what-we-built.md)** - What we actually implemented, the architecture, and current state
- **[strategic-analysis.md](./strategic-analysis.md)** - The hard question: is this worth it?
- **[architecture-research.md](./architecture-research.md)** - Research on near-workspaces, nearcore runtime, and alternatives

## TL;DR

We built a working in-process contract testing runtime (~1265 lines, 25 passing tests). It can:
- Deploy contracts, execute calls, handle cross-contract calls with callbacks
- Run 10-100x faster than near-workspaces (no subprocess, no RPC)

But we're pausing to ask: is this the right approach? The gap with nearcore is significant, and `near-vm-runner` isn't a great primitive to build on.

See [strategic-analysis.md](./strategic-analysis.md) for the full discussion.
