#!/usr/bin/env node
'use strict';

const crypto = require('crypto');
const fs = require('fs');
const path = require('path');

const packageMetadata = require('../package.json');

const { TARGETS, getArchive: archiveName } = require('../launcher/config.cjs');

function fileSha256(filePath) {
  return crypto.createHash('sha256').update(fs.readFileSync(filePath)).digest('hex');
}

function generateManifest(artifactsDir) {
  const targets = {};
  for (const target of TARGETS) {
    const archive = archiveName(target);
    const filePath = path.join(artifactsDir, archive);
    if (!fs.statSync(filePath).isFile()) {
      throw new Error(`Native archive is not a file: ${filePath}`);
    }
    targets[target] = { archive, sha256: fileSha256(filePath) };
  }
  return {
    schema: 1,
    package: packageMetadata.name,
    version: packageMetadata.version,
    targets,
  };
}

function main() {
  const artifactsDir = path.resolve(process.argv[2] || '../../dist');
  const output = path.resolve(process.argv[3] || 'native-manifest.json');
  const manifest = generateManifest(artifactsDir);
  fs.writeFileSync(output, `${JSON.stringify(manifest, null, 2)}\n`);
  console.log(`Wrote ${output}`);
}

if (require.main === module) main();

module.exports = { TARGETS, archiveName, generateManifest };
