#!/usr/bin/env node
'use strict';

const { spawnSync } = require('child_process');
const fs = require('fs');
const os = require('os');
const path = require('path');

const VERSION = require('./package.json').version;
if (!VERSION) {
  throw new Error('Missing or invalid version in package.json');
}
const REPO = '1nder-labs/schemalint';

const TARGET_MAP = {
  'darwin-x64': 'x86_64-apple-darwin',
  'darwin-arm64': 'aarch64-apple-darwin',
  'linux-x64': 'x86_64-unknown-linux-gnu',
  'linux-arm64': 'aarch64-unknown-linux-gnu',
  'win32-x64': 'x86_64-pc-windows-msvc',
};

const EXE_EXT = process.platform === 'win32' ? '.exe' : '';

function getTarget() {
  const plat = `${process.platform}-${process.arch}`;
  const target = TARGET_MAP[plat];
  if (!target) {
    throw new Error(
      `Unsupported platform: ${plat}. Supported: ${Object.keys(TARGET_MAP).join(', ')}`
    );
  }
  return target;
}

function getCacheDir() {
  const dir = process.env.SCHEMALINT_CACHE_DIR
    || path.join(os.homedir(), '.cache', 'schemalint-npm');
  return path.join(dir, VERSION);
}

function getBinaryPath() {
  const target = getTarget();
  return path.join(getCacheDir(), target, `schemalint${EXE_EXT}`);
}

function ensureBinary() {
  const binPath = getBinaryPath();
  const sentinelPath = binPath + '.verified';

  if (fs.existsSync(binPath) && fs.existsSync(sentinelPath)) {
    return binPath;
  }

  // Cache miss or incomplete extraction — clean slate.
  try { fs.unlinkSync(sentinelPath); } catch {}
  try { fs.unlinkSync(binPath); } catch {}

  const target = getTarget();
  const ext = process.platform === 'win32' ? '.zip' : '.tar.gz';
  const archiveName = `schemalint-${target}${ext}`;
  const url = `https://github.com/${REPO}/releases/download/v${VERSION}/${archiveName}`;
  const cacheDir = path.dirname(binPath);
  fs.mkdirSync(cacheDir, { recursive: true });

  const archivePath = path.join(cacheDir, archiveName);

  const { execSync } = require('child_process');

  // Download with retry (exponential backoff, max 3 attempts).
  let lastError = null;
  for (let attempt = 1; attempt <= 3; attempt++) {
    try {
      execSync(`curl -fsSL --connect-timeout 10 --max-time 120 -o "${archivePath}" "${url}"`, { stdio: 'pipe' });
      lastError = null;
      break;
    } catch (e) {
      lastError = e.stderr ? e.stderr.toString().trim() : e.message;
      if (attempt < 3) {
        try { fs.unlinkSync(archivePath); } catch {}
        const delayMs = Math.pow(2, attempt) * 1000;
        const end = Date.now() + delayMs;
        while (Date.now() < end) { /* spin-wait; acceptable for brief CLI installer delays */ }
      }
    }
  }

  if (lastError !== null) {
    throw new Error(
      `Failed to download schemalint binary from ${url}: ${lastError}. ` +
      `Make sure the GitHub Release for v${VERSION} exists.`
    );
  }

  // Extract based on platform.
  try {
    if (process.platform === 'win32') {
      execSync(`powershell -Command "Expand-Archive -Path '${archivePath}' -DestinationPath '${cacheDir}'"`, { stdio: 'pipe' });
    } else {
      execSync(`tar -xzf "${archivePath}" -C "${cacheDir}"`, { stdio: 'pipe' });
    }
  } catch (e) {
    const reason = e.stderr ? e.stderr.toString().trim() : e.message;
    try { fs.unlinkSync(archivePath); } catch {}
    throw new Error(`Failed to extract schemalint binary: ${reason}`);
  }

  // Clean up archive.
  try { fs.unlinkSync(archivePath); } catch {}

  if (!fs.existsSync(binPath)) {
    throw new Error(`Binary not found at ${binPath} after extraction`);
  }

  // Make executable on Unix.
  if (process.platform !== 'win32') {
    fs.chmodSync(binPath, 0o755);
  }

  // Write sentinel to mark successful extraction.
  fs.writeFileSync(sentinelPath, 'verified');

  return binPath;
}

try {
  const binPath = ensureBinary();
  const result = spawnSync(binPath, process.argv.slice(2), { stdio: 'inherit' });
  process.exit(result.status ?? 1);
} catch (err) {
  console.error(`schemalint: ${err.message}`);
  process.exit(1);
}
