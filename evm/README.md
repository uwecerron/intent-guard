# EVM notes

The EVM implementation uses two contracts:

- `IntentGuardModule.sol`: queue, veto, and execute proposals through a Safe.
- `CollateralListingAdapter.sol`: example action adapter.

## Important

The module is not protective if the Safe can still call guarded targets directly. Production deployments need one of:

- guarded protocol ownership transferred to the module,
- a Safe Guard that blocks direct calls to covered functions,
- protected contracts that require `msg.sender == IntentGuardModule` for guarded functions.

## Signature format

The module verifies signatures over:

```text
vaultId
nonce
target
value
dataHash
intentHash
adapter
signedAt
expiresAt
chainId
module
```

Signers must submit addresses sorted ascending to prevent duplicate-signature ambiguity.
