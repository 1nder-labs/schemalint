'use strict';

const fs = require('fs');
const https = require('https');

const MAX_DOWNLOAD_BYTES = 200 * 1024 * 1024;
const MAX_REDIRECTS = 5;
const ALLOWED_HOSTS = new Set([
  'github.com',
  'objects.githubusercontent.com',
  'release-assets.githubusercontent.com',
]);

function assertAllowedUrl(url) {
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
    throw new Error(`Refusing download from disallowed host "${parsed.hostname}"`);
  }
  return parsed;
}

function redirectUrl(location, depth) {
  if (depth >= MAX_REDIRECTS) {
    throw new Error(`Too many redirects; maximum is ${MAX_REDIRECTS}`);
  }
  assertAllowedUrl(location);
  return location;
}

function downloadFile(url, destination, depth = 0) {
  assertAllowedUrl(url);
  return new Promise((resolve, reject) => {
    const request = https.get(url, (response) => {
      if (response.statusCode >= 300 && response.statusCode < 400) {
        response.resume();
        try {
          const next = redirectUrl(response.headers.location || '', depth);
          downloadFile(next, destination, depth + 1).then(resolve, reject);
        } catch (error) {
          reject(error);
        }
        return;
      }
      if (response.statusCode !== 200) {
        response.resume();
        reject(new Error(`HTTP ${response.statusCode}`));
        return;
      }

      const declared = Number.parseInt(response.headers['content-length'] || '0', 10);
      if (declared > MAX_DOWNLOAD_BYTES) {
        response.destroy();
        reject(new Error(`Download exceeds ${MAX_DOWNLOAD_BYTES}-byte cap`));
        return;
      }

      let received = 0;
      let settled = false;
      const file = fs.createWriteStream(destination, { flags: 'wx' });
      const fail = (error) => {
        if (settled) return;
        settled = true;
        request.destroy();
        file.destroy();
        fs.unlink(destination, () => {});
        reject(error);
      };
      response.on('data', (chunk) => {
        received += chunk.length;
        if (received > MAX_DOWNLOAD_BYTES) {
          fail(new Error(`Download exceeds ${MAX_DOWNLOAD_BYTES}-byte cap`));
        }
      });
      response.on('error', fail);
      file.on('error', fail);
      file.on('finish', () => {
        file.close((error) => {
          if (error) fail(error);
          else if (!settled) {
            settled = true;
            resolve();
          }
        });
      });
      response.pipe(file);
    });
    request.on('error', (error) => {
      fs.unlink(destination, () => {});
      reject(error);
    });
    request.setTimeout(120_000, () => {
      request.destroy(new Error('Request timeout'));
    });
  });
}

async function downloadWithRetry(url, destination, attempts = 3) {
  let lastError;
  for (let attempt = 1; attempt <= attempts; attempt += 1) {
    try {
      await downloadFile(url, destination);
      return;
    } catch (error) {
      lastError = error;
      try { fs.unlinkSync(destination); } catch {}
      if (attempt < attempts) {
        await new Promise((resolve) => setTimeout(resolve, 2 ** attempt * 1000));
      }
    }
  }
  throw lastError;
}

module.exports = {
  ALLOWED_HOSTS,
  MAX_DOWNLOAD_BYTES,
  MAX_REDIRECTS,
  assertAllowedUrl,
  downloadFile,
  downloadWithRetry,
  redirectUrl,
};
