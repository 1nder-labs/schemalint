'use strict';

// postinstall: replace the Node launcher at bin/schemalint with the verified
// native binary, so every package-manager link runs schemalint with no Node
// process in front of it. Any failure leaves the launcher in place; it
// downloads the binary on first run instead.

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

const config = require('./config.cjs');
const { sha256File } = require('./integrity.cjs');

const PACKAGE_DIR = path.join(__dirname, '..');

// Only a real dependency install: not this source checkout, not a Yarn PnP
// cache, and never anything outside the package directory.
function installedInNodeModules(packageDir) {
  let parent = path.dirname(packageDir);
  if (path.basename(parent).startsWith('@')) parent = path.dirname(parent);
  return path.basename(parent) === 'node_modules';
}

function isLauncher(file) {
  const head = Buffer.alloc(2);
  const fd = fs.openSync(file, 'r');
  try {
    fs.readSync(fd, head, 0, 2, 0);
  } finally {
    fs.closeSync(fd);
  }
  return head.toString() === '#!';
}

function reportedVersion(file) {
  const result = spawnSync(file, ['--version'], { encoding: 'utf8', timeout: 30_000 });
  return result.status === 0 ? result.stdout.trim() : null;
}

async function installNativeEntry(options = {}) {
  const packageDir = options.packageDir || PACKAGE_DIR;
  const platform = options.platform || process.platform;
  const ensureBinary = options.ensureBinary || require('./ensure-binary.cjs').ensureBinary;
  // npm writes .cmd/.ps1 shims that run node on the bin file on Windows.
  if (platform === 'win32' || !installedInNodeModules(packageDir)) return 'skipped';

  const entry = path.join(packageDir, 'bin', 'schemalint');
  const temporary = `${entry}.install-tmp`;
  const expected = `schemalint ${config.VERSION}`;
  try {
    const binary = await ensureBinary();
    const verified = JSON.parse(fs.readFileSync(`${binary}.verified`, 'utf8'));
    fs.rmSync(temporary, { force: true });
    fs.copyFileSync(binary, temporary, fs.constants.COPYFILE_FICLONE);
    fs.chmodSync(temporary, 0o755);
    if (await sha256File(temporary) !== verified.binarySha256) {
      throw new Error('copied binary does not match its verified SHA-256');
    }
    const version = reportedVersion(temporary);
    if (version !== expected) throw new Error(`binary reports ${version}, expected ${expected}`);
    // rename, never write in place: pnpm hard-links package files from its store.
    fs.renameSync(temporary, entry);
    return 'native';
  } catch (error) {
    fs.rmSync(temporary, { force: true });
    if (!isLauncher(entry) && reportedVersion(entry) !== expected) {
      throw new Error(`bin/schemalint is a stale native binary and could not be replaced: ${error.message}`);
    }
    console.warn(
      `schemalint: kept the Node launcher (${error.message}); ` +
        'it will download the native binary on first run.'
    );
    return 'launcher';
  }
}

if (require.main === module) {
  installNativeEntry().catch((error) => {
    console.error(`schemalint: ${error.message}`);
    process.exitCode = 1;
  });
}

module.exports = { installNativeEntry, installedInNodeModules };
