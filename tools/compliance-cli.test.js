'use strict';

const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const cli = path.resolve(__dirname, 'compliance-cli');

test('dry-run allowlist add reads contract ID from env config', () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'compliance-cli-'));
  const config = path.join(dir, 'contracts.env');
  fs.writeFileSync(config,
    'ALLOWLIST_CONTRACT_ID=CALLOW\nDENYLIST_CONTRACT_ID=CDENY\n' +
    'JURISDICTION_CONTRACT_ID=CJUR\nSTELLAR_NETWORK=testnet\n');

  const result = spawnSync(process.execPath, [
    cli, '--config', config, '--admin', 'GADMIN', '--source', 'admin-key',
    '--dry-run', 'allowlist', 'add', 'GUSER',
  ], { encoding: 'utf8' });

  assert.equal(result.status, 0);
  assert.match(result.stdout, /--id CALLOW/);
  assert.match(result.stdout, /add_to_allowlist/);
  assert.match(result.stdout, /--address GUSER/);
});

test('denylist remove emits the denylist contract invocation', () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'compliance-cli-'));
  const config = path.join(dir, 'contracts.json');
  fs.writeFileSync(config, JSON.stringify({
    denylist: { contract_id: 'CDENY' },
  }));

  const result = spawnSync(process.execPath, [
    cli, '--config', config, '--admin', 'GADMIN', '--source', 'admin-key',
    '--dry-run', 'denylist', 'remove', 'GUSER',
  ], { encoding: 'utf8' });

  assert.equal(result.status, 0);
  assert.match(result.stdout, /--id CDENY/);
  assert.match(result.stdout, /remove_from_denylist/);
});

test('jurisdiction set rejects a missing code', () => {
  const result = spawnSync(process.execPath, [
    cli, 'jurisdiction', 'set', 'GUSER',
  ], { encoding: 'utf8' });

  assert.notEqual(result.status, 0);
});
