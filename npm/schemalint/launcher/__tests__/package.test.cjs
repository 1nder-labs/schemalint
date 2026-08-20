'use strict';

const assert = require('assert');
const fs = require('fs');
const os = require('os');
const path = require('path');
const { spawnSync } = require('child_process');
const test = require('node:test');
const {
  TARGETS,
  archiveName,
  generateManifest,
} = require('../../scripts/generate-native-manifest.cjs');
const {
  validateManifest,
  validateVersions,
} = require('../../scripts/validate-native-manifest.cjs');
const packageVersion = require('../../package.json').version;

test('release manifest generation covers final archives and version parity', () => {
  const repoRoot = path.resolve(__dirname, '../../../..');
  const artifacts = fs.mkdtempSync(path.join(os.tmpdir(), 'schemalint-artifacts-'));
  for (const target of TARGETS) {
    fs.writeFileSync(path.join(artifacts, archiveName(target)), `archive:${target}`);
  }
  const manifest = generateManifest(artifacts);
  const manifestPath = path.join(artifacts, 'native-manifest.json');
  fs.writeFileSync(manifestPath, JSON.stringify(manifest));

  assert.equal(Object.keys(validateManifest(manifestPath, true).targets).length, 5);
  assert.doesNotThrow(() => validateVersions(repoRoot, `v${packageVersion}`));
  assert.throws(() => validateVersions(repoRoot, 'v9.9.9'), /Release tag/);
});

test('packed npm inventory contains immutable manifest and focused launcher only', () => {
  const packageDir = path.resolve(__dirname, '../..');
  const destination = fs.mkdtempSync(path.join(os.tmpdir(), 'schemalint-pack-'));
  const result = spawnSync(
    'npm',
    ['pack', '--ignore-scripts', '--json', '--pack-destination', destination],
    {
      cwd: packageDir,
      encoding: 'utf8',
      env: { ...process.env, npm_config_cache: path.join(destination, 'npm-cache') },
    }
  );
  assert.equal(result.status, 0, result.stderr);
  const files = JSON.parse(result.stdout)[0].files.map((entry) => entry.path);
  assert(files.includes('native-manifest.json'));
  assert(files.includes('launcher/integrity.cjs'));
  assert(files.includes('launcher/ensure-binary.cjs'));
  assert(!files.some((file) => file.includes('__tests__')));
});
