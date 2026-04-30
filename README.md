# intentguard

Intentguard is an on-chain guardrail for privileged DeFi operations. It makes a dangerous action prove:

1. what the signers intended,
2. what the transaction actually does,
3. whether the signatures are fresh,
4. whether the action survived a public cool-off window,
5. whether oracle-sensitive claims match allowlisted live data.

This repository currently contains a design paper plus reference implementation scaffolds for EVM and Solana. Treat the code as a starting point for review and protocol-specific integration, not an audited drop-in security product.

## Layout

- `intentguard.md` — whitepaper and threat model.
- `contracts/IntentGuardModule.sol` — EVM Safe-compatible guarded execution module.
- `contracts/CollateralListingAdapter.sol` — example adapter for a collateral-listing action.
- `solana/programs/intentguard/src/lib.rs` — Anchor-style Solana program scaffold.
- `signer-cli/src/intent.ts` — TypeScript helpers for intent and attestation hashes.

## One-sentence description

Intentguard is an on-chain guardrail that makes every dangerous protocol action prove what signers meant, what the transaction does, and whether the approval is still fresh before it can execute.

## EVM integration

1. Deploy `IntentGuardModule`.
2. Initialize a vault from the Safe with the Safe address, signer set, threshold, veto threshold, freshness window, cool-off, execute delay, and proposal lifetime.
3. Deploy protocol-specific adapters for each guarded action.
4. From the Safe, allow each `(target, adapter)` pair.
5. Move ownership of guarded protocol functions to the module, install a Safe Guard that blocks direct calls, or update protected contracts to require calls from the module.
6. Have signers sign the attestation payload over vault, nonce, target, calldata hash, intent hash, adapter, chain ID, module address, and expiry.
7. Queue the proposal with fresh attestations.
8. Monitor the queue during the cool-off.
9. Execute only after the cool-off and execute-delay windows pass, unless enough signers cancel first.

The most important EVM integration rule: enabling a Safe module does not stop Safe owners from bypassing it. You must remove or block direct Safe/admin execution for covered actions.

## Solana integration

1. Deploy the Anchor program in `solana/programs/intentguard`.
2. Initialize one vault per guarded authority.
3. Transfer guarded authority to the vault PDA.
4. Build protocol-specific adapters that decode target instructions, recompute canonical intent hashes, and perform oracle/risk checks.
5. Have signers produce ed25519 attestations over the vault, proposal, nonce, target program, adapter, intent hash, instruction data hash, and expiry.
6. Queue proposals only while attestations are fresh.
7. Monitor queued proposals and use `cancel` if anything is wrong.
8. Execute via CPI after the cool-off and execute delay.

The current Solana program includes the state machine and attestation shape, but adapter validation is intentionally left as an integration point because Solana protocols differ heavily in account layouts and instruction formats.

## Production checklist

- Direct bypass is impossible for covered actions.
- Every action has a deterministic adapter.
- Unknown actions fail closed.
- Signatures bind chain/domain, vault, nonce, target, data hash, intent hash, adapter, signer, and expiry.
- Freshness is checked before queueing.
- The queue emits alerts and is monitored outside the signing UI.
- Veto threshold is calibrated for both incident response and liveness.
- Oracle feeds are allowlisted, fresh, and hard to spoof.
- Thin or new collateral has strict caps or fails closed.
- Adapter and oracle changes use longer delays than routine parameter changes.
- Cross-chain verification has separate defenses: multi-verifier configs, RPC quorum reads, source-chain proofs where possible, and circuit breakers.

## Status

Unaudited. Do not deploy as-is to secure funds.
