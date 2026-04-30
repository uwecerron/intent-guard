# Intent schema notes

Intentguard only works if each guarded action has a canonical schema. A schema must define:

- `action_kind`
- target program or contract
- all action parameters visible to signers
- all risk-relevant derived values
- oracle/feed identities
- exposure caps
- expiry
- adapter version

## Example: collateral listing

```text
WhitelistCollateral {
  target: ProtocolRiskManager,
  token: CVT,
  oracle: Pyth/CVT-USD or Chainlink/CVT-USD,
  fair_value_usd: 1.00,
  max_deposit_usd: 500_000,
  max_ltv_bps: 0,
  liquidation_threshold_bps: 0,
  adapter_version: 1,
  expires_at: 2026-04-30T18:00:00Z
}
```

## Adapter rule

The adapter must decode the actual call or instruction and recompute the same canonical object. If any field is missing, ambiguous, derived from an untrusted source, or impossible to verify, the adapter should fail closed.

## Oracle rule

Governance should not be able to choose arbitrary feeds at proposal time. Feed identity should come from an allowlist or risk adapter. For thin assets, the safest default is to reject the listing or set very small exposure caps until independent market data exists.
