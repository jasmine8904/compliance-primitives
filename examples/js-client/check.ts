/**
 * Minimal TypeScript client example — calls denylist-gate.check(address)
 * against a deployed Soroban contract using @stellar/stellar-sdk.
 *
 * Usage:
 *   cp .env.example .env          # fill in contract ID and address
 *   npm install
 *   npm run check
 *
 * Or pass values via environment variables directly:
 *   DENYLIST_GATE_CONTRACT_ID=C... ADDRESS_TO_CHECK=G... npm run check
 */

import {
  Contract,
  Networks,
  nativeToScVal,
  scValToNative,
  SorobanRpc,
  TransactionBuilder,
  Account,
  BASE_FEE,
  Address,
  xdr,
} from "@stellar/stellar-sdk";
import { readFileSync, existsSync } from "node:fs";

// ---------------------------------------------------------------------------
// Load environment (from .env file if present, else process.env)
// ---------------------------------------------------------------------------

function loadEnv(): void {
  const envPath = new URL(".env", import.meta.url).pathname;
  if (!existsSync(envPath)) return;
  const lines = readFileSync(envPath, "utf-8").split("\n");
  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;
    const eq = trimmed.indexOf("=");
    if (eq === -1) continue;
    const key = trimmed.slice(0, eq).trim();
    const val = trimmed.slice(eq + 1).trim().replace(/^"|"$/g, "");
    if (!(key in process.env)) process.env[key] = val;
  }
}

loadEnv();

const CONTRACT_ID = process.env.DENYLIST_GATE_CONTRACT_ID ?? "";
const ADDRESS_TO_CHECK = process.env.ADDRESS_TO_CHECK ?? "";
const CIRCUIT_BREAKER_CONTRACT_ID =
  process.env.CIRCUIT_BREAKER_CONTRACT_ID ?? "";
const JURISDICTION_FLAG_CONTRACT_ID =
  process.env.JURISDICTION_FLAG_CONTRACT_ID ?? "";
const RPC_URL =
  process.env.STELLAR_RPC_URL ?? "https://soroban-testnet.stellar.org";
const NETWORK_PASSPHRASE =
  process.env.NETWORK_PASSPHRASE ?? Networks.TESTNET;

if (!CONTRACT_ID || !ADDRESS_TO_CHECK) {
  console.error(
    "Error: DENYLIST_GATE_CONTRACT_ID and ADDRESS_TO_CHECK must be set.\n" +
      "Copy .env.example to .env and fill in the values, or set them as\n" +
      "environment variables before running this script."
  );
  process.exit(1);
}

// ---------------------------------------------------------------------------
// Call denylist-gate.check(address) — a read-only simulation
// ---------------------------------------------------------------------------

async function main(): Promise<void> {
  const server = new SorobanRpc.Server(RPC_URL);

  // For a read-only simulation we don't need a funded account. We use the
  // contract address itself as a dummy source — the simulation engine only
  // requires a structurally valid transaction, not a funded one.
  const source = new Account(ADDRESS_TO_CHECK, "0");

  const contract = new Contract(CONTRACT_ID);

  // Build the transaction invoking check(address_to_check).
  // `check` takes a single Soroban Address argument.
  const tx = new TransactionBuilder(source, {
    fee: BASE_FEE,
    networkPassphrase: NETWORK_PASSPHRASE,
  })
    .addOperation(
      contract.call(
        "check",
        nativeToScVal(Address.fromString(ADDRESS_TO_CHECK), { type: "address" })
      )
    )
    .setTimeout(30)
    .build();

  console.log(`RPC endpoint : ${RPC_URL}`);
  console.log(`Contract     : ${CONTRACT_ID}`);
  console.log(`Address      : ${ADDRESS_TO_CHECK}`);
  console.log();

  // Simulate — no signing or submission needed for a read-only call.
  const result = await server.simulateTransaction(tx);

  if (SorobanRpc.Api.isSimulationError(result)) {
    console.error("Simulation failed:", result.error);
    process.exit(1);
  }

  if (!SorobanRpc.Api.isSimulationSuccess(result)) {
    console.error("Unexpected simulation response:", result);
    process.exit(1);
  }

  // The return value is a ScVal (XDR). Convert it to a JS native value.
  const returnVal = result.result?.retval;
  if (returnVal === undefined) {
    console.error("No return value in simulation result");
    process.exit(1);
  }

  const isAllowed: boolean = scValToNative(returnVal) as boolean;

  console.log(
    `denylist-gate.check(${ADDRESS_TO_CHECK}) => ${isAllowed ? "✅ allowed" : "🚫 denied"}`
  );
}

// ---------------------------------------------------------------------------
// Pre-transfer compliance helper
//
// Wallets should call this before building a transfer. It performs the
// read-only checks required by the compliance contracts and returns a
// structured result the UI can render:
//
//   1. circuit-breaker.is_frozen()            — global pause switch
//   2. denylist-gate.check(from)              — sender not denylisted
//   3. denylist-gate.check(to)                — recipient not denylisted
//   4. jurisdiction-flag.is_permitted_jurisdiction(from)
//   5. jurisdiction-flag.is_permitted_jurisdiction(to)
//
// Any ContractError returned by the contracts is mapped to a user-facing
// message via `complianceMessage`.
// ---------------------------------------------------------------------------

/** ContractError variants surfaced by the compliance contracts. */
export enum ContractError {
  Frozen = 1,
  Denylisted = 2,
  JurisdictionNotPermitted = 3,
  Unknown = 99,
}

export interface PreTransferResult {
  allowed: boolean;
  /** User-facing message suitable for display in a wallet UI. */
  message: string;
  /** Machine-readable error code, if the transfer is blocked. */
  error?: ContractError;
}

/**
 * Map a ContractError (or raw error string from simulation) to a
 * user-facing compliance message.
 */
export function complianceMessage(error: ContractError | string): string {
  const code =
    typeof error === "string" ? parseContractError(error) : error;
  switch (code) {
    case ContractError.Frozen:
      return "Transfers are temporarily paused by the issuer. Please try again later.";
    case ContractError.Denylisted:
      return "One of the parties is on the issuer's denylist and cannot send or receive this asset.";
    case ContractError.JurisdictionNotPermitted:
      return "One of the parties is in a jurisdiction where this asset is not permitted.";
    default:
      return "This transfer failed a compliance check. Contact the asset issuer for details.";
  }
}

/** Best-effort extraction of a ContractError code from a simulation error. */
function parseContractError(raw: string): ContractError {
  const match = raw.match(/Error\(Contract,\s*#?(\d+)\)/);
  if (match) {
    const code = Number(match[1]);
    if (code in ContractError) return code as ContractError;
  }
  return ContractError.Unknown;
}

/** Simulate a read-only contract call and return the native result. */
async function simulateCall(
  server: SorobanRpc.Server,
  contractId: string,
  method: string,
  args: xdr.ScVal[],
  sourceAddress: string
): Promise<unknown> {
  const source = new Account(sourceAddress, "0");
  const contract = new Contract(contractId);
  const tx = new TransactionBuilder(source, {
    fee: BASE_FEE,
    networkPassphrase: NETWORK_PASSPHRASE,
  })
    .addOperation(contract.call(method, ...args))
    .setTimeout(30)
    .build();

  const result = await server.simulateTransaction(tx);
  if (SorobanRpc.Api.isSimulationError(result)) {
    throw new Error(result.error);
  }
  if (!SorobanRpc.Api.isSimulationSuccess(result)) {
    throw new Error("Unexpected simulation response");
  }
  const retval = result.result?.retval;
  if (retval === undefined) {
    throw new Error("No return value in simulation result");
  }
  return scValToNative(retval);
}

/**
 * Run all pre-transfer compliance checks for a transfer from `from` to `to`.
 *
 * @param from Sender Stellar address (G...)
 * @param to   Recipient Stellar address (G...)
 */
export async function preTransferCheck(
  from: string,
  to: string
): Promise<PreTransferResult> {
  const server = new SorobanRpc.Server(RPC_URL);
  const addressArg = (addr: string) =>
    nativeToScVal(Address.fromString(addr), { type: "address" });

  try {
    // 1. Global pause switch.
    if (CIRCUIT_BREAKER_CONTRACT_ID) {
      const frozen = (await simulateCall(
        server,
        CIRCUIT_BREAKER_CONTRACT_ID,
        "is_frozen",
        [],
        from
      )) as boolean;
      if (frozen) {
        return {
          allowed: false,
          error: ContractError.Frozen,
          message: complianceMessage(ContractError.Frozen),
        };
      }
    }

    // 2. Denylist checks for both parties.
    if (CONTRACT_ID) {
      for (const party of [from, to]) {
        const allowed = (await simulateCall(
          server,
          CONTRACT_ID,
          "check",
          [addressArg(party)],
          from
        )) as boolean;
        if (!allowed) {
          return {
            allowed: false,
            error: ContractError.Denylisted,
            message: complianceMessage(ContractError.Denylisted),
          };
        }
      }
    }

    // 3. Jurisdiction checks for both parties.
    if (JURISDICTION_FLAG_CONTRACT_ID) {
      for (const party of [from, to]) {
        const permitted = (await simulateCall(
          server,
          JURISDICTION_FLAG_CONTRACT_ID,
          "is_permitted_jurisdiction",
          [addressArg(party)],
          from
        )) as boolean;
        if (!permitted) {
          return {
            allowed: false,
            error: ContractError.JurisdictionNotPermitted,
            message: complianceMessage(
              ContractError.JurisdictionNotPermitted
            ),
          };
        }
      }
    }

    return { allowed: true, message: "Transfer passed all compliance checks." };
  } catch (err) {
    const raw = err instanceof Error ? err.message : String(err);
    const code = parseContractError(raw);
    return { allowed: false, error: code, message: complianceMessage(code) };
  }
}

main().catch((err) => {
  console.error("Unexpected error:", err);
  process.exit(1);
});
