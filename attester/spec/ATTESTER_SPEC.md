# intentguard attester spec

Version: 0.1 draft
Status: design scaffold

## 1. Problem

Intentguard verifies that the transaction matches a typed intent, but the signer's laptop may still lie about the rendered intent.

The attester addresses this by moving rendering and approval to a separate device with:

- audited renderers,
- a narrow schema registry,
- a separate signing key,
- physical confirmation,
- firmware and renderer identity in the signed payload.

## 2. Threat model

### Defends against

- compromised signer laptop,
- malicious browser extension,
- fake dapp frontend,
- misleading RPC simulation result,
- clipboard or QR tampering by the host,
- raw calldata presented as a friendly action,
- stale intent shown to the human while a different proposal is submitted.

### Does not defend against

- malicious firmware,
- malicious schema bundle,
- supply-chain compromise,
- human approving a clearly malicious rendered intent,
- total signer and attester collusion,
- protocol adapter bug on-chain,
- oracle/feed compromise.

## 3. Core objects

- **Schema bundle:** signed set of action schemas and renderers.
- **Renderer:** deterministic code that converts canonical intent bytes to display pages.
- **Display digest:** hash of the exact pages shown to the user.
- **Attester key:** device-held key used only for intent attestation.
- **Firmware hash:** identity of the firmware build.
- **Renderer hash:** identity of the renderer bundle.
- **Device attestation:** proof that the attester key belongs to approved firmware/device class.

## 4. Attestation payload

Canonical payload:

```text
AttesterApproval {
  domain: "intentguard.attester.v1",
  chain_id_or_genesis_hash,
  vault,
  proposal_id,
  nonce,
  action_kind,
  target,
  data_hash,
  intent_hash,
  schema_id,
  schema_version,
  renderer_hash,
  firmware_hash,
  display_digest,
  signed_at,
  expires_at
}
```

EVM encoding: typed ABI encoding plus `keccak256`.

Solana encoding: deterministic Borsh or another fixed canonical encoding plus SHA-256.

## 5. Rendering rules

The attester must display:

- protocol name,
- chain,
- vault,
- action kind,
- target,
- decoded arguments,
- risk flags,
- oracle/feed identity,
- caps and limits,
- recipient/admin addresses,
- nonce,
- expiry,
- schema version.

The attester must refuse:

- unknown schema,
- unknown action kind,
- unknown target,
- missing address-book entry for sensitive targets,
- oversized payload,
- non-canonical encoding,
- renderer/schema hash mismatch,
- expired proposal,
- impossible timestamp,
- unsupported chain.

## 6. Display digest

The device computes:

```text
display_digest = H(canonical_pages)
```

where `canonical_pages` are the exact text pages shown to the human, including line breaks and pagination.

The display digest lets external reviewers reproduce what the device claims it showed.

## 7. On-chain verification

Intentguard should optionally require:

```text
M signer approvals
AND
M_attester attester approvals
```

The guard checks:

- attester is registered for the vault,
- attester approval is fresh,
- attester approval binds the same `intent_hash`,
- attester approval binds the same `data_hash`,
- renderer and firmware hashes are approved,
- schema version is approved,
- attester has not been revoked.

## 8. Registration

Each vault maintains:

- allowed attester public keys,
- approved firmware hashes,
- approved renderer hashes,
- approved schema versions,
- attester quorum.

Adding or removing any of these should itself be a guarded action with a longer cool-off.

## 9. MVP implementation

Recommended first build:

1. TypeScript or Rust schema renderer.
2. Offline host client.
3. QR-code transport.
4. Mobile secure-enclave key where available.
5. EVM verifier support in `IntentGuardModule`.
6. Solana verifier support in the Anchor program.
7. Test vectors for each schema.

Only after that should a dedicated hardware device be attempted.

## 10. Open questions

- Which secure-enclave attestation APIs are acceptable across iOS and Android?
- How should firmware hashes be approved and rotated?
- What is the minimal display size for complex DeFi intents?
- How should address books be distributed and signed?
- Can renderer libraries be formally tested against schema fixtures?
- Should attester quorum differ from signer quorum?
- How should lost attester devices be revoked without creating an emergency bypass?
