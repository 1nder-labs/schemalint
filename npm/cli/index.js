#!/usr/bin/env node
'use strict';

const { spawnSync } = require('child_process');
const crypto = require('crypto');
const fs = require('fs');
const https = require('https');
const os = require('os');
const path = require('path');

const VERSION = '0.1.0';
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

function downloadFile(url, dest) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest);
    https.get(url, (res) => {
      if (res.statusCode === 302 || res.statusCode === 301) {
        file.close();
        fs.unlinkSync(dest);
        return downloadFile(res.headers.location, dest).then(resolve).catch(reject);
      }
      if (res.statusCode !== 200) {
        file.close();
        fs.unlinkSync(dest);
        return reject(new Error(`Download failed with status ${res.statusCode}`));
      }
      res.pipe(file);
      file.on('finish', () => {
        file.close();
        resolve();
      });
      file.on('error', (err) => {
        fs.unlinkSync(dest);
        reject(err);
      });
    }).on('error', (err) => {
      fs.unlinkSync(dest);
      reject(err);
    });
  });
}

function ensureBinary() {
  const binPath = getBinaryPath();
  if (fs.existsSync(binPath)) {
    return binPath;
  }

  const target = getTarget();
  const ext = process.platform === 'win32' ? '.zip' : '.tar.gz';
  const archiveName = `schemalint-${target}${ext}`;
  const url = `https://github.com/${REPO}/releases/download/v${VERSION}/${archiveName}`;
  const cacheDir = path.dirname(binPath);
  fs.mkdirSync(cacheDir, { recursive: true });

  const archivePath = path.join(cacheDir, archiveName);

  try {
    // Download archive.
    const { execSync } = require('child_process');
    execSync(`curl -fsSL -o "${archivePath}" "${url}"`, { stdio: 'pipe' });
  } catch {
    throw new Error(
      `Failed to download schemalint binary from ${url}. ` +
      `Make sure the GitHub Release for v${VERSION} exists.`
    );
  }

  // Extract based on platform.
  try {
    const { execSync } = require('child_process');
    if (process.platform === 'win32') {
      execSync(`powershell -Command "Expand-Archive -Path '${archivePath}' -DestinationPath '${cacheDir}'"`, { stdio: 'pipe' });
    } else {
      execSync(`tar -xzf "${archivePath}" -C "${cacheDir}"`, { stdio: 'pipe' });
    }
  } catch {
    throw new Error('Failed to extract schemalint binary');
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
