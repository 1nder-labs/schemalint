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
const INSTALL_LOCK_POLL_MS = 100;
const INSTALL_LOCK_STALE_MS = 10 * 60_000;
const INSTALL_LOCK_TIMEOUT_MS = 10 * 60_000;

function delay(milliseconds) {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}

function lockOwnerIsAlive(lockPath) {
  try {
    const { pid } = JSON.parse(fs.readFileSync(lockPath, 'utf8'));
    if (!Number.isSafeInteger(pid) || pid <= 0) return false;
    process.kill(pid, 0);
    return true;
  } catch (error) {
    return error.code === 'EPERM';
  }
}

async function acquireInstallLock(lockPath, options = {}) {
  const pollMs = options.pollMs ?? INSTALL_LOCK_POLL_MS;
  const staleMs = options.staleMs ?? INSTALL_LOCK_STALE_MS;
  const timeoutMs = options.timeoutMs ?? INSTALL_LOCK_TIMEOUT_MS;
  const token = `${process.pid}.${crypto.randomBytes(12).toString('hex')}`;
  const startedAt = Date.now();

  while (true) {
    try {
      fs.writeFileSync(
        lockPath,
        JSON.stringify({ token, pid: process.pid, createdAt: Date.now() }),
        { flag: 'wx', mode: 0o600 }
      );
      return () => {
        try {
          const owner = JSON.parse(fs.readFileSync(lockPath, 'utf8'));
          if (owner.token === token) fs.unlinkSync(lockPath);
        } catch {}
      };
    } catch (error) {
      if (error.code !== 'EEXIST') throw error;
    }

    try {
      if (
        Date.now() - fs.statSync(lockPath).mtimeMs >= staleMs
        && !lockOwnerIsAlive(lockPath)
      ) {
        fs.unlinkSync(lockPath);
        continue;
      }
    } catch (error) {
      if (error.code === 'ENOENT') continue;
      throw error;
    }

    const elapsed = Date.now() - startedAt;
    if (elapsed >= timeoutMs) {
      throw new Error(`Timed out waiting ${timeoutMs}ms for install lock ${lockPath}`);
    }
    await delay(Math.min(pollMs, timeoutMs - elapsed));
  }
}

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
  const archiveName = config.getArchive(target);
  const binaryPath = overrides.binaryPath || config.getBinaryPath(target, platform);
  const sentinelPath = `${binaryPath}.verified`;
  const manifest = overrides.manifest
    || loadManifest(config.MANIFEST_PATH, config.PACKAGE);
  const expectedArchiveHash = targetDigest(manifest, target, archiveName);

  if (await readVerifiedCache(binaryPath, sentinelPath, expectedArchiveHash)) {
    return binaryPath;
  }

  const cacheDir = path.dirname(binaryPath);
  fs.mkdirSync(cacheDir, { recursive: true });
  const releaseLock = await acquireInstallLock(
    `${binaryPath}.lock`,
    overrides.lockOptions
  );

  try {
    if (await readVerifiedCache(binaryPath, sentinelPath, expectedArchiveHash)) {
      return binaryPath;
    }
    cleanCache(binaryPath, sentinelPath);

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
  } finally {
    releaseLock();
  }
}

module.exports = { acquireInstallLock, ensureBinary, readVerifiedCache };
