#!/usr/bin/env node
'use strict';

const fs = require('fs');
const path = require('path');

const packageMetadata = require('../package.json');
const { loadManifest, targetDigest } = require('../launcher/integrity.cjs');
const { TARGETS, archiveName } = require('./generate-native-manifest.cjs');

function captureVersion(filePath, pattern, label) {
  const match = fs.readFileSync(filePath, 'utf8').match(pattern);
  if (!match) throw new Error(`Unable to read ${label} version from ${filePath}`);
  return match[1];
}

function validateVersions(repoRoot, tag) {
  const cargoVersion = captureVersion(
    path.join(repoRoot, 'Cargo.toml'),
    /\[workspace\.package\][\s\S]*?\nversion\s*=\s*"([^"]+)"/,
    'Cargo workspace'
  );
  const pythonVersion = captureVersion(
    path.join(repoRoot, 'crates/schemalint-python/pyproject.toml'),
    /\[project\][\s\S]*?\nversion\s*=\s*"([^"]+)"/,
    'Python package'
  );
  for (const [label, version] of [
    ['Cargo workspace', cargoVersion],
    ['Python package', pythonVersion],
  ]) {
    if (version !== packageMetadata.version) {
      throw new Error(`${label} version ${version} != npm ${packageMetadata.version}`);
    }
  }
  if (tag && tag !== `v${packageMetadata.version}`) {
    throw new Error(`Release tag ${tag} != v${packageMetadata.version}`);
  }
}

function validateManifest(manifestPath, release = false) {
  const manifest = loadManifest(manifestPath, packageMetadata);
  if (release && manifest.development) {
    throw new Error('Development native manifest cannot be released');
  }
  for (const target of TARGETS) {
    targetDigest(manifest, target, archiveName(target));
  }
  const extra = Object.keys(manifest.targets).filter((target) => !TARGETS.includes(target));
  if (extra.length) throw new Error(`Unexpected native manifest targets: ${extra.join(', ')}`);
  return manifest;
}

function main() {
  const release = process.argv.includes('--release');
  const versionsOnly = process.argv.includes('--versions-only');
  const tagIndex = process.argv.indexOf('--tag');
  const tag = tagIndex === -1 ? process.env.RELEASE_TAG : process.argv[tagIndex + 1];
  const repoRoot = path.resolve(__dirname, '../../..');
  validateVersions(repoRoot, tag);
  if (!versionsOnly) {
    validateManifest(path.join(__dirname, '..', 'native-manifest.json'), release);
  }
  console.log(
    `Validated release version ${packageMetadata.version}` +
      (versionsOnly ? '' : ' and native manifest')
  );
}

if (require.main === module) main();

module.exports = { validateManifest, validateVersions };
