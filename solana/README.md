# Solana notes

The Solana implementation is an Anchor-style program scaffold for:

- vault setup,
- proposal queueing,
- fresh signer attestations,
- K-of-N cancellation,
- delayed execution by CPI.

## Important

The program currently includes only a minimal structural check for ed25519 instruction contents. Before production use, replace `verify_ed25519_instruction` with a strict parser for Solana's ed25519 instruction header offsets.

Adapter validation is also intentionally protocol-specific. A real integration should decode the exact target instruction, recompute the canonical intent hash, and perform oracle/risk checks before CPI execution.
