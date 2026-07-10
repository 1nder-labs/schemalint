'use strict';

const crypto = require('crypto');
const fs = require('fs');
const path = require('path');

const config = require('./config.cjs');
const { downloadWithRetry } = require('./download.cjs');
const { extractArchive, selectExtractedBinary } = require('./extract.cjs');
const {
  SHA256_PATTERN,
  loadManifest,
  sha256File,
  targetDigest,
  verifyArchive,
} = require('./integrity.cjs');

let temporaryCounter = 0;

async function readVerifiedCache(binaryPath, sentinelPath, archiveHash) {
  if (!fs.existsSync(binaryPath) || !fs.existsSync(sentinelPath)) return false;
  let sentinel;
  try {
    sentinel = JSON.parse(fs.readFileSync(sentinelPath, 'utf8'));
  } catch {
    return false;
  }
  if (
    sentinel.archiveSha256 !== archiveHash
    || !SHA256_PATTERN.test(sentinel.binarySha256 || '')
  ) {
    return false;
  }
  try {
    return await sha256File(binaryPath) === sentinel.binarySha256;
  } catch {
    return false;
  }
}

function cleanCache(binaryPath, sentinelPath) {
  try { fs.unlinkSync(sentinelPath); } catch {}
  try { fs.unlinkSync(binaryPath); } catch {}
}

async function ensureBinary(overrides = {}) {
  const target = overrides.target || config.getTarget();
  const platform = overrides.platform || process.platform;
  const archiveName = config.getArchive(target, platform);
  const binaryPath = overrides.binaryPath || config.getBinaryPath(target, platform);
  const sentinelPath = `${binaryPath}.verified`;
  const manifest = overrides.manifest
    || loadManifest(config.MANIFEST_PATH, config.PACKAGE);
  const expectedArchiveHash = targetDigest(manifest, target, archiveName);

  if (await readVerifiedCache(binaryPath, sentinelPath, expectedArchiveHash)) {
    return binaryPath;
  }
  cleanCache(binaryPath, sentinelPath);

  const cacheDir = path.dirname(binaryPath);
  fs.mkdirSync(cacheDir, { recursive: true });
  const workDir = path.join(
    path.dirname(cacheDir),
    `schemalint-tmp.${process.pid}.${++temporaryCounter}.` +
      crypto.randomBytes(4).toString('hex')
  );
  fs.mkdirSync(workDir, { recursive: true });
  const archivePath = path.join(workDir, archiveName);
  const archiveUrl =
    `https://github.com/${config.REPO}/releases/download/v${config.VERSION}/${archiveName}`;
  const download = overrides.download || downloadWithRetry;
  const extract = overrides.extract || extractArchive;

  try {
    await download(archiveUrl, archivePath);
    await verifyArchive(archivePath, expectedArchiveHash);
    extract(archivePath, workDir, platform);
    try { fs.unlinkSync(archivePath); } catch {}

    const binaryName = platform === 'win32' ? 'schemalint.exe' : 'schemalint';
    const extractedBinary = selectExtractedBinary(workDir, binaryName);
    if (platform !== 'win32') fs.chmodSync(extractedBinary, 0o755);
    fs.renameSync(extractedBinary, binaryPath);

    const binarySha256 = await sha256File(binaryPath);
    fs.writeFileSync(
      sentinelPath,
      JSON.stringify({ archiveSha256: expectedArchiveHash, binarySha256 })
    );
    return binaryPath;
  } catch (error) {
    cleanCache(binaryPath, sentinelPath);
    throw new Error(`Unable to install schemalint ${config.VERSION}: ${error.message}`);
  } finally {
    try { fs.rmSync(workDir, { recursive: true, force: true }); } catch {}
  }
}

module.exports = { ensureBinary, readVerifiedCache };
