'use strict';

const os = require('os');
const path = require('path');

const PACKAGE_PATH = path.join(__dirname, '..', 'package.json');
const MANIFEST_PATH = path.join(__dirname, '..', 'native-manifest.json');
const PACKAGE = require(PACKAGE_PATH);
const VERSION = PACKAGE.version;
const REPO = '1nder-labs/schemalint';

const TARGET_MAP = Object.freeze({
  'darwin-x64': 'x86_64-apple-darwin',
  'darwin-arm64': 'aarch64-apple-darwin',
  'linux-x64': 'x86_64-unknown-linux-gnu',
  'linux-arm64': 'aarch64-unknown-linux-gnu',
  'win32-x64': 'x86_64-pc-windows-msvc',
});

function getTarget(platform = process.platform, arch = process.arch) {
  const key = `${platform}-${arch}`;
  const target = TARGET_MAP[key];
  if (!target) {
    throw new Error(
      `Unsupported platform: ${key}. Supported: ${Object.keys(TARGET_MAP).join(', ')}`
    );
  }
  return target;
}

function getArchive(target, platform = process.platform) {
  const extension = platform === 'win32' ? '.zip' : '.tar.gz';
  return `schemalint-${target}${extension}`;
}

function getCacheDir(version = VERSION) {
  const root = process.env.SCHEMALINT_CACHE_DIR
    || path.join(os.homedir(), '.cache', 'schemalint-npm');
  return path.join(root, version);
}

function getBinaryPath(target = getTarget(), platform = process.platform) {
  const extension = platform === 'win32' ? '.exe' : '';
  return path.join(getCacheDir(), target, `schemalint${extension}`);
}

module.exports = {
  MANIFEST_PATH,
  PACKAGE,
  REPO,
  TARGET_MAP,
  VERSION,
  getArchive,
  getBinaryPath,
  getCacheDir,
  getTarget,
};
