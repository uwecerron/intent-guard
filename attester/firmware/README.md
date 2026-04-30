# Firmware requirements

This folder is a placeholder for future attester firmware.

The firmware should be boring, small, and hard to misuse.

## Requirements

- secure boot,
- reproducible builds,
- signed firmware releases,
- device-held attester key,
- no arbitrary web content,
- no arbitrary transaction signing,
- deterministic schema renderer,
- physical confirmation before signing,
- firmware hash included in every attestation,
- schema bundle hash included in every attestation,
- fail-closed parsing.

## Non-goals

- general wallet functionality,
- token transfers,
- NFT display,
- browser support,
- arbitrary contract interaction,
- hidden expert modes.

## First milestone

Before firmware exists, implement the same interface in a locked-down software attester so the protocol and on-chain verifier can be tested.
