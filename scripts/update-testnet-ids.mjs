#!/usr/bin/env node
/**
 * scripts/update-testnet-ids.mjs
 *
 * Renders the deployed testnet contract IDs into README.md so developers can
 * try the contracts without deploying their own.
 *
 * Source of truth: `testnet-ids.json` at the repository root. The deploy
 * scripts write this file; its shape is:
 *
 *   {
 *     "v1.0.0": {
 *       "registry": "CABC...",
 *       "token": "CDEF..."
 *     },
 *     "v1.1.0": {
 *       "registry": "CGHI..."
 *     }
 *   }
 *
 * i.e. a mapping of release -> contract name -> contract ID.
 *
 * The script replaces everything between the following markers in README.md:
 *
 *   <!-- BEGIN TESTNET IDS -->
 *   ...
 *   <!-- END TESTNET IDS -->
 *
 * with a Markdown table sorted by release (ascending) and then by contract
 * name (ascending).
 *
 * Usage:
 *   node scripts/update-testnet-ids.mjs           # rewrite README.md
 *   node scripts/update-testnet-ids.mjs --check   # exit 1 if README is stale
 *
 * Options:
 *   --check        Do not write; exit non-zero if the README table is out of
 *                  date. Intended for CI.
 *   --readme <p>   Path to README.md (default: ./README.md).
 *   --ids <p>      Path to testnet-ids.json (default: ./testnet-ids.json).
 *   --help         Show this help.
 *
 * Exit codes:
 *   0  success (or, with --check, README is up to date)
 *   1  --check found the README out of date
 *   2  usage / IO / parse error
 */

import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const BEGIN_MARKER = "<!-- BEGIN TESTNET IDS -->";
const END_MARKER = "<!-- END TESTNET IDS -->";

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, "..");

function parseArgs(argv) {
  const opts = {
    check: false,
    readme: resolve(repoRoot, "README.md"),
    ids: resolve(repoRoot, "testnet-ids.json"),
    help: false,
  };

  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    switch (arg) {
      case "--check":
        opts.check = true;
        break;
      case "--readme":
        opts.readme = resolve(process.cwd(), argv[++i] ?? "");
        break;
      case "--ids":
        opts.ids = resolve(process.cwd(), argv[++i] ?? "");
        break;
      case "--help":
      case "-h":
        opts.help = true;
        break;
      default:
        throw new Error(`Unknown argument: ${arg}`);
    }
  }

  return opts;
}

function printHelp() {
  process.stdout.write(
    [
      "Usage: node scripts/update-testnet-ids.mjs [options]",
      "",
      "Options:",
      "  --check        Exit non-zero if README.md is out of date (no write).",
      "  --readme <p>   Path to README.md (default: ./README.md).",
      "  --ids <p>      Path to testnet-ids.json (default: ./testnet-ids.json).",
      "  --help, -h     Show this help.",
      "",
    ].join("\n"),
  );
}

function loadIds(path) {
  if (!existsSync(path)) {
    throw new Error(`testnet-ids.json not found at ${path}`);
  }
  const raw = readFileSync(path, "utf8");
  let data;
  try {
    data = JSON.parse(raw);
  } catch (err) {
    throw new Error(`Failed to parse ${path}: ${err.message}`);
  }
  if (data === null || typeof data !== "object" || Array.isArray(data)) {
    throw new Error(`${path} must contain a JSON object mapping release -> contract -> id`);
  }
  return data;
}

function escapeCell(value) {
  return String(value).replace(/\|/g, "\\|").replace(/\r?\n/g, " ");
}

function renderTable(ids) {
  const releases = Object.keys(ids).sort((a, b) => a.localeCompare(b));

  const lines = [];
  lines.push("| Release | Contract | Testnet Contract ID |");
  lines.push("| --- | --- | --- |");

  if (releases.length === 0) {
    lines.push("| _none_ | _none_ | _none_ |");
    return lines.join("\n");
  }

  for (const release of releases) {
    const contracts = ids[release];
    if (contracts === null || typeof contracts !== "object" || Array.isArray(contracts)) {
      throw new Error(`Release "${release}" must map contract name -> contract ID`);
    }
    const names = Object.keys(contracts).sort((a, b) => a.localeCompare(b));
    if (names.length === 0) {
      lines.push(`| ${escapeCell(release)} | _none_ | _none_ |`);
      continue;
    }
    for (const name of names) {
      lines.push(
        `| ${escapeCell(release)} | ${escapeCell(name)} | \`${escapeCell(contracts[name])}\` |`,
      );
    }
  }

  return lines.join("\n");
}

function replaceSection(readme, table) {
  const beginIdx = readme.indexOf(BEGIN_MARKER);
  const endIdx = readme.indexOf(END_MARKER);

  if (beginIdx === -1 || endIdx === -1) {
    throw new Error(
      `README.md is missing the ${BEGIN_MARKER} / ${END_MARKER} markers`,
    );
  }
  if (endIdx < beginIdx) {
    throw new Error(`README.md has ${END_MARKER} before ${BEGIN_MARKER}`);
  }

  const before = readme.slice(0, beginIdx + BEGIN_MARKER.length);
  const after = readme.slice(endIdx);

  return `${before}\n\n${table}\n\n${after}`;
}

function main() {
  let opts;
  try {
    opts = parseArgs(process.argv.slice(2));
  } catch (err) {
    process.stderr.write(`${err.message}\n`);
    printHelp();
    process.exit(2);
  }

  if (opts.help) {
    printHelp();
    return;
  }

  let ids;
  let readme;
  try {
    ids = loadIds(opts.ids);
    if (!existsSync(opts.readme)) {
      throw new Error(`README not found at ${opts.readme}`);
    }
    readme = readFileSync(opts.readme, "utf8");
  } catch (err) {
    process.stderr.write(`${err.message}\n`);
    process.exit(2);
  }

  let updated;
  try {
    updated = replaceSection(readme, renderTable(ids));
  } catch (err) {
    process.stderr.write(`${err.message}\n`);
    process.exit(2);
  }

  if (opts.check) {
    if (updated !== readme) {
      process.stderr.write(
        "README.md testnet IDs are out of date. Run `node scripts/update-testnet-ids.mjs` and commit the result.\n",
      );
      process.exit(1);
    }
    process.stdout.write("README.md testnet IDs are up to date.\n");
    return;
  }

  if (updated !== readme) {
    writeFileSync(opts.readme, updated);
    process.stdout.write(`Updated ${opts.readme}\n`);
  } else {
    process.stdout.write(`${opts.readme} already up to date.\n`);
  }
}

main();