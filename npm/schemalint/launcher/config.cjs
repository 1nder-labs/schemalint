'use strict';

const os = require('os');
const path = require('path');

const PACKAGE_PATH = path.join(__dirname, '..', 'package.json');
const MANIFEST_PATH = path.join(__dirname, '..', 'native-manifest.json');
const PACKAGE = require(PACKAGE_PATH);
const VERSION = PACKAGE.version;
const REPO = '1nder-labs/schemalint';

const NATIVE_TARGETS = Object.freeze([
  { platform: 'darwin', arch: 'x64', target: 'x86_64-apple-darwin', extension: '.tar.gz' },
  { platform: 'darwin', arch: 'arm64', target: 'aarch64-apple-darwin', extension: '.tar.gz' },
  { platform: 'linux', arch: 'x64', target: 'x86_64-unknown-linux-gnu', extension: '.tar.gz' },
  { platform: 'linux', arch: 'arm64', target: 'aarch64-unknown-linux-gnu', extension: '.tar.gz' },
  { platform: 'win32', arch: 'x64', target: 'x86_64-pc-windows-msvc', extension: '.zip' },
].map(Object.freeze));
const TARGETS = Object.freeze(NATIVE_TARGETS.map(({ target }) => target));
const TARGET_MAP = Object.freeze(Object.fromEntries(
  NATIVE_TARGETS.map(({ platform, arch, target }) => [`${platform}-${arch}`, target])
));

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

function getArchive(target = getTarget()) {
  const metadata = NATIVE_TARGETS.find((candidate) => candidate.target === target);
  if (!metadata) throw new Error(`Unsupported native target: ${target}`);
  return `schemalint-${target}${metadata.extension}`;
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
  NATIVE_TARGETS,
  PACKAGE,
  REPO,
  TARGET_MAP,
  TARGETS,
  VERSION,
  getArchive,
  getBinaryPath,
  getCacheDir,
  getTarget,
};
