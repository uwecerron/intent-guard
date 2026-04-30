import { encodeAbiParameters, keccak256, parseAbiParameters, stringToBytes, toBytes, type Hex } from "viem";

export type CollateralIntent = {
  target: Hex;
  value: bigint;
  token: Hex;
  oracle: Hex;
  fairValueUsdE18: bigint;
  maxDepositUsdE18: bigint;
};

export type EvmAttestationPayload = {
  vaultId: Hex;
  nonce: bigint;
  target: Hex;
  value: bigint;
  dataHash: Hex;
  intentHash: Hex;
  adapter: Hex;
  signedAt: bigint;
  expiresAt: bigint;
  chainId: bigint;
  module: Hex;
};

export function collateralIntentHash(intent: CollateralIntent): Hex {
  return keccak256(
    encodeAbiParameters(
      parseAbiParameters(
        "bytes32 typeHash, address target, uint256 value, address token, address oracle, uint256 fairValueUsdE18, uint256 maxDepositUsdE18",
      ),
      [
        keccak256(stringToBytes("WhitelistCollateral(address target,uint256 value,address token,address oracle,uint256 fairValueUsdE18,uint256 maxDepositUsdE18)")),
        intent.target,
        intent.value,
        intent.token,
        intent.oracle,
        intent.fairValueUsdE18,
        intent.maxDepositUsdE18,
      ],
    ),
  );
}

export function evmAttestationDigest(payload: EvmAttestationPayload): Hex {
  const typeHash = keccak256(
    stringToBytes(
      "IntentGuardAttestation(bytes32 vaultId,uint64 nonce,address target,uint256 value,bytes32 dataHash,bytes32 intentHash,address adapter,uint64 signedAt,uint64 expiresAt,uint256 chainId,address module)",
    ),
  );

  const structHash = keccak256(
    encodeAbiParameters(
      parseAbiParameters(
        "bytes32 typeHash, bytes32 vaultId, uint64 nonce, address target, uint256 value, bytes32 dataHash, bytes32 intentHash, address adapter, uint64 signedAt, uint64 expiresAt, uint256 chainId, address module",
      ),
      [
        typeHash,
        payload.vaultId,
        payload.nonce,
        payload.target,
        payload.value,
        payload.dataHash,
        payload.intentHash,
        payload.adapter,
        payload.signedAt,
        payload.expiresAt,
        payload.chainId,
        payload.module,
      ],
    ),
  );

  return keccak256(new Uint8Array([...stringToBytes("\x19Ethereum Signed Message:\n32"), ...toBytes(structHash)]));
}

export function solanaAttestationMessage(input: {
  vault: Uint8Array;
  proposal: Uint8Array;
  nonce: bigint;
  targetProgram: Uint8Array;
  adapter: Uint8Array;
  intentHash: Uint8Array;
  instructionDataHash: Uint8Array;
  signedAt: bigint;
  expiresAt: bigint;
}): Uint8Array {
  const parts = [
    new TextEncoder().encode("intentguard.solana.attestation.v1"),
    input.vault,
    input.proposal,
    u64Le(input.nonce),
    input.targetProgram,
    input.adapter,
    input.intentHash,
    input.instructionDataHash,
    i64Le(input.signedAt),
    i64Le(input.expiresAt),
  ];

  return concat(parts);
}

function u64Le(value: bigint): Uint8Array {
  const out = new Uint8Array(8);
  new DataView(out.buffer).setBigUint64(0, value, true);
  return out;
}
function i64Le(value: bigint): Uint8Array {
  const out = new Uint8Array(8);
  new DataView(out.buffer).setBigInt64(0, value, true);
  return out;
}

function concat(parts: Uint8Array[]): Uint8Array {
  const total = parts.reduce((sum, part) => sum + part.length, 0);
  const out = new Uint8Array(total);
  let offset = 0;
  for (const part of parts) {
    out.set(part, offset);
    offset += part.length;
  }
  return out;
}
