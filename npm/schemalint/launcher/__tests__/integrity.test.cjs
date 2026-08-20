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
const {
  acquireInstallLock,
  ensureBinary,
  readVerifiedCache,
} = require('../ensure-binary.cjs');

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

test('concurrent cold installs share one verified binary and sentinel', async () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'schemalint-concurrent-'));
  const source = path.join(dir, ARCHIVE);
  fs.writeFileSync(source, 'trusted concurrent archive');
  const hash = await sha256File(source);
  const binaryPath = path.join(dir, 'cache', TARGET, 'schemalint');
  let downloads = 0;
  let extractions = 0;
  let releaseDownload;
  const downloadReleased = new Promise((resolve) => { releaseDownload = resolve; });
  let markDownloadStarted;
  const downloadStarted = new Promise((resolve) => { markDownloadStarted = resolve; });
  const options = {
    target: TARGET,
    platform: 'linux',
    binaryPath,
    manifest: manifest(hash),
    lockOptions: { pollMs: 1, timeoutMs: 1_000 },
    download: async (_url, destination) => {
      downloads += 1;
      markDownloadStarted();
      await downloadReleased;
      fs.copyFileSync(source, destination);
    },
    extract: (_archive, destination) => {
      extractions += 1;
      fs.writeFileSync(path.join(destination, 'schemalint'), 'retained binary');
    },
  };

  const installs = Array.from({ length: 8 }, () => ensureBinary(options));
  await downloadStarted;
  releaseDownload();
  const installed = await Promise.all(installs);

  assert.deepEqual(installed, Array(8).fill(binaryPath));
  assert.equal(downloads, 1);
  assert.equal(extractions, 1);
  assert.equal(fs.readFileSync(binaryPath, 'utf8'), 'retained binary');
  assert.equal(await readVerifiedCache(binaryPath, `${binaryPath}.verified`, hash), true);
  assert.equal(fs.existsSync(`${binaryPath}.lock`), false);
});

test('install lock recovers stale owners and bounds live-owner waits', async () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'schemalint-lock-'));
  const lockPath = path.join(dir, 'schemalint.lock');
  fs.writeFileSync(lockPath, '{"token":"abandoned"}');
  const staleTime = new Date(Date.now() - 10_000);
  fs.utimesSync(lockPath, staleTime, staleTime);

  const release = await acquireInstallLock(lockPath, {
    pollMs: 1,
    staleMs: 1,
    timeoutMs: 100,
  });
  assert.notEqual(JSON.parse(fs.readFileSync(lockPath, 'utf8')).token, 'abandoned');
  release();
  assert.equal(fs.existsSync(lockPath), false);

  fs.writeFileSync(lockPath, '{"token":"live"}');
  await assert.rejects(
    acquireInstallLock(lockPath, { staleMs: 60_000, timeoutMs: 0 }),
    /Timed out waiting 0ms/
  );

  fs.writeFileSync(lockPath, JSON.stringify({ token: 'slow', pid: process.pid }));
  fs.utimesSync(lockPath, staleTime, staleTime);
  await assert.rejects(
    acquireInstallLock(lockPath, { staleMs: 1, timeoutMs: 0 }),
    /Timed out waiting 0ms/
  );
  assert.equal(JSON.parse(fs.readFileSync(lockPath, 'utf8')).token, 'slow');
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
