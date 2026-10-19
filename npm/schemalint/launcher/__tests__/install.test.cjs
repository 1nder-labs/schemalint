'use strict';

const assert = require('assert');
const crypto = require('crypto');
const fs = require('fs');
const os = require('os');
const path = require('path');
const { spawnSync } = require('child_process');
const test = require('node:test');

const { installNativeEntry, installedInNodeModules } = require('../install.cjs');
const { VERSION } = require('../config.cjs');

const LAUNCHER = '#!/usr/bin/env node\nimport("../index.cjs");\n';
const unix = { skip: process.platform === 'win32' };

function sha256(file) {
  return crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
}

// A stand-in "native" binary in a verified cache, as ensureBinary leaves it.
function cachedBinary(binary, version = VERSION) {
  fs.mkdirSync(path.dirname(binary), { recursive: true });
  fs.writeFileSync(binary, `#!/bin/sh\necho "schemalint ${version}"\n`, { mode: 0o755 });
  fs.writeFileSync(`${binary}.verified`, JSON.stringify({
    archiveSha256: 'a'.repeat(64),
    binarySha256: sha256(binary),
  }));
  return binary;
}

function packageDir(root) {
  const dir = path.join(root, 'node_modules', '@1nder-labs', 'schemalint');
  fs.mkdirSync(path.join(dir, 'bin'), { recursive: true });
  fs.writeFileSync(path.join(dir, 'bin', 'schemalint'), LAUNCHER, { mode: 0o755 });
  return dir;
}

const tempRoot = () => fs.mkdtempSync(path.join(os.tmpdir(), 'schemalint-install-'));
const entryOf = (dir) => fs.readFileSync(path.join(dir, 'bin', 'schemalint'), 'utf8');

test('only scoped or unscoped node_modules package dirs qualify', () => {
  assert(installedInNodeModules('/p/node_modules/@1nder-labs/schemalint'));
  assert(installedInNodeModules('/p/node_modules/schemalint'));
  assert(!installedInNodeModules('/repo/npm/schemalint'));
  assert(!installedInNodeModules('/yarn/cache/@1nder-labs/schemalint'));
});

test('the verified binary replaces the launcher', unix, async () => {
  const root = tempRoot();
  const binary = cachedBinary(path.join(root, 'cache', 'schemalint'));
  const dir = packageDir(root);
  const state = await installNativeEntry({ packageDir: dir, ensureBinary: async () => binary });
  assert.equal(state, 'native');
  assert.equal(sha256(path.join(dir, 'bin', 'schemalint')), sha256(binary));
  assert.deepEqual(fs.readdirSync(path.join(dir, 'bin')), ['schemalint']);
});

test('Windows and source checkouts keep the launcher untouched', async () => {
  const root = tempRoot();
  const dir = packageDir(root);
  const ensureBinary = async () => assert.fail('must not install');
  assert.equal(await installNativeEntry({ packageDir: dir, platform: 'win32', ensureBinary }), 'skipped');
  assert.equal(await installNativeEntry({ packageDir: path.join(root, 'src'), ensureBinary }), 'skipped');
  assert.equal(entryOf(dir), LAUNCHER);
});

test('download, digest, or version failures keep the launcher', unix, async () => {
  const root = tempRoot();
  const dir = packageDir(root);
  const failures = [
    async () => { throw new Error('offline'); },
    async () => cachedBinary(path.join(root, 'old', 'schemalint'), '0.0.1'),
    async () => {
      const binary = cachedBinary(path.join(root, 'tampered', 'schemalint'));
      fs.appendFileSync(binary, '# tampered\n');
      return binary;
    },
  ];
  for (const ensureBinary of failures) {
    assert.equal(await installNativeEntry({ packageDir: dir, ensureBinary }), 'launcher');
    assert.equal(entryOf(dir), LAUNCHER);
    assert.deepEqual(fs.readdirSync(path.join(dir, 'bin')), ['schemalint']);
  }
});

test('an older native entry that cannot be replaced fails the install', unix, async () => {
  const root = tempRoot();
  const dir = packageDir(root);
  fs.writeFileSync(path.join(dir, 'bin', 'schemalint'), 'not a launcher', { mode: 0o755 });
  await assert.rejects(
    installNativeEntry({ packageDir: dir, ensureBinary: async () => { throw new Error('offline'); } }),
    /stale native binary/
  );
});

test('the real postinstall script swaps the entry inside node_modules', unix, () => {
  const root = tempRoot();
  const dir = packageDir(root);
  const source = path.resolve(__dirname, '../..');
  fs.cpSync(path.join(source, 'launcher'), path.join(dir, 'launcher'), {
    recursive: true,
    filter: (file) => !file.includes('__tests__'),
  });
  fs.copyFileSync(path.join(source, 'package.json'), path.join(dir, 'package.json'));
  const { getArchive, getTarget } = require('../config.cjs');
  const target = getTarget();
  fs.writeFileSync(path.join(dir, 'native-manifest.json'), JSON.stringify({
    schema: 1,
    package: '@1nder-labs/schemalint',
    version: VERSION,
    targets: { [target]: { archive: getArchive(target), sha256: 'a'.repeat(64) } },
  }));
  const cacheRoot = path.join(root, 'cache-root');
  cachedBinary(path.join(cacheRoot, VERSION, target, 'schemalint'));

  const result = spawnSync(process.execPath, [path.join(dir, 'launcher', 'install.cjs')], {
    encoding: 'utf8',
    env: { ...process.env, SCHEMALINT_CACHE_DIR: cacheRoot },
  });
  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.stderr, '');
  const entry = path.join(dir, 'bin', 'schemalint');
  assert.equal(spawnSync(entry, ['--version'], { encoding: 'utf8' }).stdout.trim(), `schemalint ${VERSION}`);
});
