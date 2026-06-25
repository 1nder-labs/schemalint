#!/usr/bin/env node
'use strict';

const { spawnSync } = require('child_process');
const crypto = require('crypto');
const fs = require('fs');
const https = require('https');
const os = require('os');
const path = require('path');

const VERSION = require('./package.json').version;
if (!VERSION) {
  throw new Error('Missing or invalid version in package.json');
}
const REPO = '1nder-labs/schemalint';

// [P0 — supply chain] Allowlist of hosts permitted for downloads and redirects.
// Only GitHub release delivery hosts are trusted; any other host (including
// same-registrable-domain lookalikes) is rejected before a connection is made.
const ALLOWED_HOSTS = new Set([
  'github.com',
  'objects.githubusercontent.com',
  'release-assets.githubusercontent.com',
  'codeload.github.com',
]);

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

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Compute the SHA-256 hash of a file and return it as a lowercase hex string.
 */
function sha256File(filePath) {
  return new Promise((resolve, reject) => {
    const hash = crypto.createHash('sha256');
    const stream = fs.createReadStream(filePath);
    stream.on('data', (chunk) => hash.update(chunk));
    stream.on('end', () => resolve(hash.digest('hex')));
    stream.on('error', reject);
  });
}

/**
 * [P0 — supply chain] Assert that a URL's hostname is in the allowed set.
 * Parses with the WHATWG URL API so scheme, port, and hostname are all
 * properly separated — substring/endsWith matching on the raw string is
 * deliberately avoided to prevent `github.com.evil.com`-style bypasses.
 * Throws for unparseable URLs or relative Location headers (fail closed).
 */
function assertAllowedHost(url) {
  let parsed;
  try {
    parsed = new URL(url);
  } catch {
    throw new Error(`Refusing download: could not parse URL: ${url}`);
  }
  if (parsed.protocol !== 'https:') {
    throw new Error(`Refusing non-HTTPS URL: ${url}`);
  }
  if (!ALLOWED_HOSTS.has(parsed.hostname)) {
    throw new Error(
      `Refusing download from disallowed host "${parsed.hostname}". ` +
      `Allowed hosts: ${[...ALLOWED_HOSTS].join(', ')}`
    );
  }
}

function downloadFile(url, destPath, depth = 0) {
  // [P0 — supply chain] Validate initial URL and every redirect target before
  // opening a connection. assertAllowedHost also enforces HTTPS, making the
  // old ternary that conditionally required 'http' unnecessary.
  assertAllowedHost(url);

  return new Promise((resolve, reject) => {
    const request = https.get(url, (response) => {
      if (response.statusCode >= 300 && response.statusCode < 400 && response.headers.location) {
        response.resume();
        if (depth > 5) {
          reject(new Error('too many redirects'));
          return;
        }
        // assertAllowedHost is called at the top of the recursive call, which
        // covers both the https-only and host-allowlist checks for the redirect.
        // The try/catch converts any synchronous throw (e.g. disallowed host,
        // unparseable Location) into a rejection rather than an uncaught exception
        // escaping the response callback.
        try {
          downloadFile(response.headers.location, destPath, depth + 1).then(resolve).catch(reject);
        } catch (e) {
          reject(e);
        }
        return;
      }
      if (response.statusCode !== 200) {
        response.resume();
        reject(new Error(`HTTP ${response.statusCode}`));
        return;
      }
      const file = fs.createWriteStream(destPath);
      response.pipe(file);
      file.on('finish', () => {
        file.close((err) => {
          if (err) reject(err);
          else resolve();
        });
      });
      file.on('error', (err) => {
        file.destroy();
        fs.unlink(destPath, () => {});
        reject(err);
      });
      response.on('error', (err) => {
        file.destroy();
        fs.unlink(destPath, () => {});
        reject(err);
      });
    });
    request.on('error', (err) => {
      fs.unlink(destPath, () => {});
      reject(err);
    });
    request.setTimeout(120000, () => {
      request.destroy();
      fs.unlink(destPath, () => {});
      reject(new Error('Request timeout'));
    });
  });
}

async function ensureBinary() {
  const binPath = getBinaryPath();
  const sentinelPath = binPath + '.verified';

  // [P0 — integrity] Cache-hit path: re-verify the cached binary against the
  // SHA-256 recorded in the sentinel on every invocation. An empty, unreadable,
  // or malformed sentinel is treated as a mismatch and triggers a fresh
  // download+verify cycle.
  if (fs.existsSync(binPath) && fs.existsSync(sentinelPath)) {
    let recordedHash = '';
    try {
      recordedHash = fs.readFileSync(sentinelPath, 'utf8').trim();
    } catch {
      // Unreadable sentinel — treat as mismatch; fall through.
    }
    if (/^[0-9a-f]{64}$/.test(recordedHash)) {
      let actualHash = '';
      try {
        actualHash = await sha256File(binPath);
      } catch {
        // Unreadable binary — fall through to fresh download.
      }
      if (actualHash === recordedHash) {
        return binPath;
      }
      // Hash mismatch or unreadable binary: wipe both so the clean-slate
      // unlinks below complete a tidy state before re-downloading.
    }
    // Sentinel absent, empty, malformed, or hash mismatch — fall through.
  }

  // Cache miss or failed integrity check — clean slate.
  try { fs.unlinkSync(sentinelPath); } catch {}
  try { fs.unlinkSync(binPath); } catch {}

  const target = getTarget();
  const ext = process.platform === 'win32' ? '.zip' : '.tar.gz';
  const archiveName = `schemalint-${target}${ext}`;
  const url = `https://github.com/${REPO}/releases/download/v${VERSION}/${archiveName}`;
  const cacheDir = path.dirname(binPath);
  fs.mkdirSync(cacheDir, { recursive: true });

  const archivePath = path.join(cacheDir, archiveName);

  // Download with retry (exponential backoff, max 3 attempts).
  let lastError = null;
  for (let attempt = 1; attempt <= 3; attempt++) {
    try {
      await downloadFile(url, archivePath);
      lastError = null;
      break;
    } catch (e) {
      lastError = e.message;
      if (attempt < 3) {
        try { fs.unlinkSync(archivePath); } catch {}
        const delayMs = Math.pow(2, attempt) * 1000;
        await sleep(delayMs);
      }
    }
  }

  if (lastError !== null) {
    throw new Error(
      `Failed to download schemalint binary from ${url}: ${lastError}. ` +
      `Make sure the GitHub Release for v${VERSION} exists.`
    );
  }

  // Verify archive integrity against the published per-artifact SHA-256 checksum.
  // cargo-dist (v0.31.0) emits a <archive>.sha256 sidecar for every archive and
  // uploads it alongside the archive in the GitHub Release.
  const checksumUrl = `${url}.sha256`;
  const checksumPath = archivePath + '.sha256';
  try {
    // Download checksum with retry (same backoff shape as archive download).
    let checksumLastError = null;
    for (let attempt = 1; attempt <= 3; attempt++) {
      try {
        await downloadFile(checksumUrl, checksumPath);
        checksumLastError = null;
        break;
      } catch (e) {
        checksumLastError = e.message;
        if (attempt < 3) {
          try { fs.unlinkSync(checksumPath); } catch {}
          const delayMs = Math.pow(2, attempt) * 1000;
          await sleep(delayMs);
        }
      }
    }
    if (checksumLastError !== null) {
      throw new Error(
        `Failed to download checksum from ${checksumUrl}: ${checksumLastError}`
      );
    }

    const checksumContent = fs.readFileSync(checksumPath, 'utf8');
    // Handles both bare-hash and "hash  filename" formats.
    const expectedHash = checksumContent.trim().split(/\s+/)[0].toLowerCase();
    const actualHash = await sha256File(archivePath);
    if (actualHash !== expectedHash) {
      throw new Error(
        `SHA-256 mismatch for ${archiveName}: expected ${expectedHash}, got ${actualHash}`
      );
    }
  } catch (e) {
    // Clean up stale archive so a failed verify doesn't leave it on disk.
    try { fs.unlinkSync(archivePath); } catch {}
    // Re-throw integrity failures unconditionally (fail closed).
    throw e;
  } finally {
    try { fs.unlinkSync(checksumPath); } catch {}
  }

  // Extract based on platform.
  try {
    if (process.platform === 'win32') {
      const result = spawnSync(
        'powershell',
        ['-Command', 'Expand-Archive', '-Path', archivePath, '-DestinationPath', cacheDir],
        { stdio: 'pipe' }
      );
      if (result.error) throw result.error;
      if (result.status !== 0) throw new Error(result.stderr.toString().trim());
    } else {
      const result = spawnSync(
        'tar',
        ['-xzf', archivePath, '-C', cacheDir],
        { stdio: 'pipe' }
      );
      if (result.error) throw result.error;
      if (result.status !== 0) throw new Error(result.stderr.toString().trim());
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

  // [P0 — path traversal] Verify the extracted binary is inside cacheDir.
  // Both sides are resolved through realpathSync so that symlinks (e.g. macOS
  // /tmp → /private/tmp) do not cause false rejections. The archive is already
  // checksum-verified at this point, but we still guard defensively because
  // Tar-Slip / Zip-Slip attacks target the extraction step.
  // Note: GNU tar `--no-absolute-filenames` is deliberately NOT used because
  // macOS ships bsdtar, which does not recognise that long option and would
  // break every macOS install. The realpath containment check is the portable
  // guard for both tar and Expand-Archive paths.
  try {
    const realBin = fs.realpathSync(binPath);
    const realCacheDir = fs.realpathSync(cacheDir);
    if (!realBin.startsWith(realCacheDir + path.sep)) {
      try { fs.unlinkSync(binPath); } catch {}
      try { fs.unlinkSync(sentinelPath); } catch {}
      throw new Error(
        `Path traversal detected: extracted binary "${realBin}" escapes cache directory "${realCacheDir}"`
      );
    }
  } catch (e) {
    if (e.message.startsWith('Path traversal')) throw e;
    // realpathSync can fail if binPath disappeared between existsSync and here —
    // treat as a missing-binary error so the outer handler surfaces a clear message.
    throw new Error(`Binary not accessible after extraction: ${e.message}`);
  }

  // Make executable on Unix.
  if (process.platform !== 'win32') {
    fs.chmodSync(binPath, 0o755);
  }

  // [P0 — integrity] Write the SHA-256 of the extracted binary (not the archive)
  // into the sentinel. On future cache hits this hash is re-verified against the
  // live binary before the cached path is returned, ensuring a tampered binary is
  // caught on the very next invocation.
  const binaryHash = await sha256File(binPath);
  fs.writeFileSync(sentinelPath, binaryHash);

  return binPath;
}

(async () => {
  try {
    const binPath = await ensureBinary();
    const result = spawnSync(binPath, process.argv.slice(2), { stdio: 'inherit' });
    process.exit(result.status ?? 1);
  } catch (err) {
    console.error(`schemalint: ${err.message}`);
    process.exit(1);
  }
})();
