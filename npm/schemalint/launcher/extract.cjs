'use strict';

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

function assertSafeArchivePath(entry) {
  const normalized = entry.replaceAll('\\', '/');
  const segments = normalized.split('/');
  if (
    normalized.startsWith('/')
    || /^[A-Za-z]:\//.test(normalized)
    || segments.includes('..')
  ) {
    throw new Error(`Unsafe archive entry: ${entry}`);
  }
}

function validateTarEntries(archivePath) {
  const result = spawnSync('tar', ['-tzf', archivePath], { encoding: 'utf8' });
  if (result.error) throw result.error;
  if (result.status !== 0) {
    throw new Error(result.stderr.trim() || 'unable to list archive');
  }
  for (const entry of result.stdout.split('\n').filter(Boolean)) {
    assertSafeArchivePath(entry);
  }
}

function getSystemPowerShellPath(environment = process.env) {
  const systemRoot = environment.SystemRoot || environment.WINDIR || 'C:\\Windows';
  if (!path.win32.isAbsolute(systemRoot)) {
    throw new Error(`Windows system root must be absolute: ${systemRoot}`);
  }
  return path.win32.join(
    systemRoot,
    'System32',
    'WindowsPowerShell',
    'v1.0',
    'powershell.exe'
  );
}

function extractArchive(archivePath, destination, platform = process.platform) {
  let result;
  if (platform === 'win32') {
    result = spawnSync(
      getSystemPowerShellPath(),
      ['-NoProfile', '-Command', 'Expand-Archive', '-LiteralPath', archivePath,
        '-DestinationPath', destination],
      { encoding: 'utf8' }
    );
  } else {
    validateTarEntries(archivePath);
    result = spawnSync('tar', ['-xzf', archivePath, '-C', destination], {
      encoding: 'utf8',
    });
  }
  if (result.error) throw result.error;
  if (result.status !== 0) {
    throw new Error(result.stderr.trim() || 'archive extraction failed');
  }
}

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
    const stat = (() => {
      try { return fs.lstatSync(fullPath); } catch { return null; }
    })();
    if (!stat || stat.isSymbolicLink()) continue;
    if (stat.isFile() && entry.name === binaryName) {
      results.push(fullPath);
    } else if (stat.isDirectory() && currentDepth < maxDepth) {
      results.push(...findBinaryInDir(fullPath, binaryName, maxDepth, currentDepth + 1));
    }
  }
  return results;
}

function selectExtractedBinary(workDir, binaryName) {
  const candidates = findBinaryInDir(workDir, binaryName, 3);
  if (candidates.length !== 1) {
    throw new Error(
      `Expected exactly one extracted ${binaryName}; found ${candidates.length}`
    );
  }
  const realWorkDir = fs.realpathSync(workDir);
  const realBinary = fs.realpathSync(candidates[0]);
  if (!realBinary.startsWith(`${realWorkDir}${path.sep}`)) {
    throw new Error(`Path traversal detected: ${realBinary} escapes ${realWorkDir}`);
  }
  const stat = fs.lstatSync(realBinary);
  if (!stat.isFile() || stat.isSymbolicLink() || stat.nlink !== 1) {
    throw new Error('Extracted binary must be one regular, non-linked file');
  }
  return realBinary;
}

module.exports = {
  assertSafeArchivePath,
  extractArchive,
  findBinaryInDir,
  getSystemPowerShellPath,
  selectExtractedBinary,
  validateTarEntries,
};
