'use strict';

const crypto = require('crypto');
const fs = require('fs');

const SHA256_PATTERN = /^[0-9a-f]{64}$/;

function sha256File(filePath) {
  return new Promise((resolve, reject) => {
    const hash = crypto.createHash('sha256');
    const stream = fs.createReadStream(filePath);
    stream.on('data', (chunk) => hash.update(chunk));
    stream.on('end', () => resolve(hash.digest('hex')));
    stream.on('error', reject);
  });
}

function loadManifest(manifestPath, packageMetadata) {
  let manifest;
  try {
    manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
  } catch (error) {
    throw new Error(`Invalid packaged native manifest: ${error.message}`);
  }
  if (manifest.schema !== 1) {
    throw new Error(`Unsupported native manifest schema: ${manifest.schema}`);
  }
  if (manifest.package !== packageMetadata.name) {
    throw new Error(
      `Native manifest package mismatch: ${manifest.package} != ${packageMetadata.name}`
    );
  }
  if (manifest.version !== packageMetadata.version) {
    throw new Error(
      `Native manifest version mismatch: ${manifest.version} != ${packageMetadata.version}`
    );
  }
  if (!manifest.targets || typeof manifest.targets !== 'object') {
    throw new Error('Native manifest is missing target digests');
  }
  return manifest;
}

function targetDigest(manifest, target, archiveName) {
  const entry = manifest.targets[target];
  if (!entry) {
    throw new Error(`Native manifest has no digest for target ${target}`);
  }
  if (entry.archive !== archiveName) {
    throw new Error(
      `Native manifest archive mismatch for ${target}: ${entry.archive} != ${archiveName}`
    );
  }
  const digest = String(entry.sha256 || '').toLowerCase();
  if (!SHA256_PATTERN.test(digest)) {
    throw new Error(`Native manifest has invalid SHA-256 for target ${target}`);
  }
  return digest;
}

async function verifyArchive(archivePath, expectedHash) {
  if (!SHA256_PATTERN.test(expectedHash)) {
    throw new Error('Refusing archive verification with an invalid packaged SHA-256');
  }
  const actualHash = await sha256File(archivePath);
  if (actualHash !== expectedHash) {
    throw new Error(
      `SHA-256 mismatch for ${archivePath}: expected ${expectedHash}, got ${actualHash}`
    );
  }
  return actualHash;
}

module.exports = {
  SHA256_PATTERN,
  loadManifest,
  sha256File,
  targetDigest,
  verifyArchive,
};
