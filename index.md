---
title: intentguard
description: A primitive for closing the web2 attack vector in DeFi. Solana and EVM. Open source.
hero_label: A primitive
hero_title: Closing the web2 attack vector in DeFi.
hero_subtitle: A small on-chain gatekeeper for privileged DeFi actions on Solana and EVM. Built in response to the half-billion dollars stolen from DeFi users at Drift and Kelp in April 2026.
---

## The problem

In April 2026, more than half a billion dollars was stolen from DeFi users at Drift and Kelp. Neither incident was a smart contract bug. Both attackers exploited the gap between what signers thought they were approving and what their signature actually meant on-chain.

The Drift attack was a six-month DPRK social engineering operation. Council members were walked into pre-signing routine-looking transactions via Solana's durable nonces. When the transactions were finally broadcast, the real on-chain effect was admin handover plus the whitelisting of a worthless token at $1.00 as collateral. The attackers minted 500M of that token, deposited it, and withdrew $285M of real assets.

The system performed perfectly. The attackers won anyway.

That class of failure is what this primitive addresses.

## What intentguard does

Intentguard is a small on-chain gatekeeper that sits between a council or multisig and any privileged protocol action: admin transfers, upgrades, collateral whitelists, treasury withdrawals.

It enforces five invariants before execution.

1. **Intent binding.** Each privileged action carries a signed, machine-verifiable intent statement. The on-chain guard recomputes the actual call's intent and rejects the action if it does not match what the signers approved.
2. **Freshness window.** Signatures must be fresh when the proposal is queued (default 10 minutes). This kills the durable-nonce abuse that hit Drift.
3. **Cool-off and veto.** A 24-hour public window during which any K-of-N signers can cancel.
4. **Oracle-bound claims.** Oracle-dependent fields, such as a token's fair value, are checked against live, allowlisted feeds at execute time.
5. **Action whitelist.** Only registered action kinds with deterministic adapters can be queued. Unknown actions fail closed.

A six-month patient social engineering attack collapses into a 24-hour public confrontation. Honest signers, given visibility and time, almost always win that confrontation.

## Read

The full design, threat model, and reference implementations.

- [Whitepaper](intentguard.html)
- [How to use it](docs/HOWTO.html)
- [Source code on GitHub](https://github.com/uwecerron/intent-guard)

## Why this matters

Most DeFi users turned to crypto because the existing financial system already failed them. When DeFi gets drained, they lose their savings, not the VCs.

We built a system for the debanked and wired it to a signing UX that fails the moment a recruiter sends a Calendly link.

This is one piece of a fix.

## Status

Reference design and reference implementations. Unaudited. Open for review, forks, and audits.

If you deploy intentguard, fork it, or audit it, get in touch. If you find a flaw in the design, get in touch faster.
