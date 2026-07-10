'use strict';

const assert = require('assert');
const fs = require('fs');
const os = require('os');
const path = require('path');
const test = require('node:test');

const {
  loadManifest,
  sha256File,
  targetDigest,
  verifyArchive,
} = require('../integrity.cjs');
const { ensureBinary } = require('../ensure-binary.cjs');

const TARGET = 'x86_64-unknown-linux-gnu';
const ARCHIVE = `schemalint-${TARGET}.tar.gz`;

function manifest(hash) {
  return {
    schema: 1,
    package: '@1nder-labs/schemalint',
    version: '1.1.0',
    targets: { [TARGET]: { archive: ARCHIVE, sha256: hash } },
  };
}

test('valid archive is accepted using only the packaged digest', async () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'schemalint-valid-'));
  const source = path.join(dir, ARCHIVE);
  fs.writeFileSync(source, 'trusted archive');
  const hash = await sha256File(source);
  const requested = [];
  const binaryPath = path.join(dir, 'cache', TARGET, 'schemalint');

  const installed = await ensureBinary({
    target: TARGET,
    platform: 'linux',
    binaryPath,
    manifest: manifest(hash),
    download: async (url, destination) => {
      requested.push(url);
      fs.copyFileSync(source, destination);
    },
    extract: (_archive, destination) => {
      fs.writeFileSync(path.join(destination, 'schemalint'), 'binary');
    },
  });

  assert.equal(installed, binaryPath);
  assert.equal(requested.length, 1);
  assert(!requested[0].endsWith('.sha256'), 'remote checksum must never be requested');
});

test('a replaced remote checksum cannot authorize a replaced archive', async () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'schemalint-integrity-'));
  const archive = path.join(dir, 'schemalint-x86_64-unknown-linux-gnu.tar.gz');
  fs.writeFileSync(archive, 'trusted archive');
  const packagedHash = await sha256File(archive);

  fs.writeFileSync(archive, 'attacker archive');
  const attackerControlledRemoteHash = await sha256File(archive);
  assert.notEqual(attackerControlledRemoteHash, packagedHash);
  await assert.rejects(
    verifyArchive(archive, packagedHash),
    /SHA-256 mismatch/
  );
});

test('manifest rejects version mismatch, missing target, and malformed digest', () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'schemalint-manifest-'));
  const manifestPath = path.join(dir, 'manifest.json');
  fs.writeFileSync(manifestPath, JSON.stringify({ ...manifest('0'.repeat(64)), version: '9.9.9' }));
  assert.throws(
    () => loadManifest(manifestPath, { name: '@1nder-labs/schemalint', version: '1.1.0' }),
    /version mismatch/
  );
  assert.throws(() => targetDigest(manifest('0'.repeat(64)), 'missing', ARCHIVE), /no digest/);
  assert.throws(() => targetDigest(manifest('wrong'), TARGET, ARCHIVE), /invalid SHA-256/);
});
