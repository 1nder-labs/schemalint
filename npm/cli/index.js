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

/**
 * Recursively search `dir` for a file named `binaryName`, up to `maxDepth`
 * levels deep. Symlinked directories are NOT followed (Zip-Slip guard).
 * Returns all matching file paths (not directories, not symlinks-to-files).
 */
function findBinaryInDir(dir, binaryName, maxDepth, currentDepth = 0) {
  const results = [];
  let entries;
  try {
    entries = fs.readdirSync(dir, { withFileTypes: true });
  } catch {
    return results;
  }
  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isFile() && entry.name === binaryName) {
      results.push(fullPath);
    } else if (entry.isDirectory() && !entry.isSymbolicLink() && currentDepth < maxDepth) {
      // entry.isSymbolicLink() is always false when using withFileTypes on a
      // plain entry because isDirectory() and isSymbolicLink() are mutually
      // exclusive for Dirent. Explicitly check via lstatSync for safety.
      const stat = (() => { try { return fs.lstatSync(fullPath); } catch { return null; } })();
      if (stat && stat.isDirectory() && !stat.isSymbolicLink()) {
        results.push(...findBinaryInDir(fullPath, binaryName, maxDepth, currentDepth + 1));
      }
    }
  }
  return results;
}

// Monotonic counter to disambiguate temp dirs created within the same process
// during the same millisecond (rare but possible in test harnesses).
let _tmpCounter = 0;

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

  // Ensure the final destination directory exists before the temp-dir sibling
  // is created (both share the same parent, so mkdirSync is only called once).
  fs.mkdirSync(cacheDir, { recursive: true });

  // [Concurrency] Use a unique temp work directory that is a SIBLING of
  // cacheDir (same parent → same filesystem), so the final fs.renameSync into
  // binPath is a within-filesystem atomic operation (never EXDEV).
  // The pid + monotonic counter + random suffix make collisions practically
  // impossible even under a test harness that spawns many processes rapidly.
  const randomSuffix = crypto.randomBytes(4).toString('hex');
  const workDir = path.join(
    path.dirname(cacheDir),
    `schemalint-tmp.${process.pid}.${++_tmpCounter}.${randomSuffix}`
  );
  fs.mkdirSync(workDir, { recursive: true });

  try {
    // All temp artifacts (archive, checksum, extracted tree) live inside
    // workDir so that a crashed/killed process never leaves partial state in
    // cacheDir visible to a concurrent healthy process.
    const archivePath = path.join(workDir, archiveName);

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

    // Verify archive integrity against the published per-artifact SHA-256
    // checksum. cargo-dist (v0.31.0) emits a <archive>.sha256 sidecar for
    // every archive and uploads it alongside the archive in the GitHub Release.
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
    } finally {
      // Always remove the checksum file — it is ephemeral and must not linger.
      try { fs.unlinkSync(checksumPath); } catch {}
    }

    // Extract into workDir. Every platform extracts to the same isolated
    // temp tree; the archive is then deleted, and the binary is searched for
    // within that tree regardless of how cargo-dist laid out the subdirectory.
    try {
      if (process.platform === 'win32') {
        const result = spawnSync(
          'powershell',
          ['-Command', 'Expand-Archive', '-Path', archivePath, '-DestinationPath', workDir],
          { stdio: 'pipe' }
        );
        if (result.error) throw result.error;
        if (result.status !== 0) throw new Error(result.stderr.toString().trim());
      } else {
        const result = spawnSync(
          'tar',
          ['-xzf', archivePath, '-C', workDir],
          { stdio: 'pipe' }
        );
        if (result.error) throw result.error;
        if (result.status !== 0) throw new Error(result.stderr.toString().trim());
      }
    } catch (e) {
      const reason = e.stderr ? e.stderr.toString().trim() : e.message;
      throw new Error(`Failed to extract schemalint binary: ${reason}`);
    }

    // Archive is no longer needed once extraction succeeds.
    try { fs.unlinkSync(archivePath); } catch {}

    // [P0 — layout-agnostic binary location] cargo-dist 0.31 archives may
    // place the binary at the archive root OR inside a `schemalint-<target>/`
    // subdirectory. Search workDir recursively (bounded to depth 3) for a file
    // named `schemalint[.exe]`, then enforce exactly one match.
    const binaryName = `schemalint${EXE_EXT}`;
    const candidates = findBinaryInDir(workDir, binaryName, 3);

    if (candidates.length === 0) {
      throw new Error(
        `Binary "${binaryName}" not found anywhere inside the extracted archive. ` +
        `Searched up to depth 3 in ${workDir}.`
      );
    }
    if (candidates.length > 1) {
      throw new Error(
        `Ambiguous extraction: found ${candidates.length} files named "${binaryName}" ` +
        `inside the archive: ${candidates.join(', ')}`
      );
    }

    const foundPath = candidates[0];

    // [P0 — path traversal] Resolve the found path through realpathSync and
    // verify it is inside workDir before trusting it. This guards against
    // Tar-Slip / Zip-Slip attacks that place a symlink pointing outside the
    // extraction directory. Note: GNU tar `--no-absolute-filenames` is
    // deliberately NOT used because macOS ships bsdtar which does not
    // recognise that long option. The realpath containment check is the
    // portable guard for both tar and Expand-Archive paths.
    let realFound;
    try {
      realFound = fs.realpathSync(foundPath);
    } catch (e) {
      throw new Error(`Binary not accessible after extraction: ${e.message}`);
    }
    const realWorkDir = fs.realpathSync(workDir);
    if (!realFound.startsWith(realWorkDir + path.sep)) {
      throw new Error(
        `Path traversal detected: extracted binary "${realFound}" escapes ` +
        `work directory "${realWorkDir}"`
      );
    }

    // Make executable on Unix before moving into place.
    if (process.platform !== 'win32') {
      fs.chmodSync(realFound, 0o755);
    }

    // [Concurrency] Atomically rename the verified binary into its final
    // location. Because workDir is a sibling of cacheDir (same filesystem),
    // renameSync is always within-device and therefore atomic. A racing process
    // that already placed binPath will simply have its file replaced by an
    // equally valid binary — both came from the same verified archive.
    // The sentinel is written AFTER the rename, so the cache-hit path (which
    // requires both binPath and sentinelPath to exist) can never observe a
    // partial state left by a crash between the two writes.
    fs.renameSync(realFound, binPath);

    // [P0 — integrity] Write the SHA-256 of the installed binary into the
    // sentinel. On future cache hits this hash is re-verified against the live
    // binary before the cached path is returned, ensuring a tampered binary is
    // caught on the very next invocation.
    const binaryHash = await sha256File(binPath);
    fs.writeFileSync(sentinelPath, binaryHash);

  } finally {
    // Always clean up the temp work directory, even on error. This removes
    // any partially extracted or partially downloaded artifacts so a failed
    // run never leaves corrupt state visible to concurrent or subsequent runs.
    try { fs.rmSync(workDir, { recursive: true, force: true }); } catch {}
  }

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
