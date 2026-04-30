# Host client requirements

The host client moves proposal data between the protocol UI and the attester.

The host is not trusted.

It may:

- fetch proposals,
- package canonical intent bundles,
- send bundles by QR, USB, NFC, or local transport,
- receive attester signatures,
- submit attestations on-chain.

It must not be trusted to:

- decide what the intent means,
- rewrite schemas,
- hide fields,
- choose oracle feeds,
- change targets,
- decide whether a proposal is safe.

## Bundle format

Every host-to-attester bundle should contain:

- chain,
- vault,
- proposal id,
- nonce,
- action kind,
- target,
- calldata or instruction hash,
- intent hash,
- canonical intent bytes,
- schema id,
- schema version,
- expiry.

The attester must recompute all hashes before rendering.
