'use strict';

const { spawnSync } = require('child_process');
const fs = require('fs');
const path = require('path');

/**
 * Resolve the argv prefix used to spawn the schemalint CLI.
 *
 * Resolution order:
 *   1. Try require.resolve('@1nder-labs/cli/package.json') to find the CLI
 *      package installed alongside this package. Read its `bin.schemalint`
 *      entry to get the JS script path, then return [process.execPath, script]
 *      so the script is always spawned via the current Node binary — portable
 *      regardless of shebang support or executable bits.
 *   2. If module resolution fails (CLI not a resolvable dependency), fall back
 *      to ['schemalint'] so global installs continue to work unchanged.
 *
 * @returns {string[]} argv prefix, e.g. ['/usr/bin/node', '/path/to/cli/index.js']
 *                     or ['schemalint']
 */
function resolveCliArgv() {
  try {
    const pkgJsonPath = require.resolve('@1nder-labs/cli/package.json');
    const pkgJson = JSON.parse(fs.readFileSync(pkgJsonPath, 'utf-8'));
    const binEntry = pkgJson.bin && (pkgJson.bin.schemalint || pkgJson.bin);
    if (!binEntry || typeof binEntry !== 'string') {
      return ['schemalint'];
    }
    const cliScript = path.resolve(path.dirname(pkgJsonPath), binEntry);
    return [process.execPath, cliScript];
  } catch {
    // CLI not resolvable as a local dependency — fall back to PATH lookup.
    return ['schemalint'];
  }
}

/**
 * Shared spawn helper. Takes the resolved argv prefix (from resolveCliArgv)
 * and merges it with the subcommand args.
 *
 * When argv is [node, script], we spawnSync(node, [script, ...args]).
 * When argv is ['schemalint'],  we spawnSync('schemalint', args) — identical
 * to the original behaviour.
 */
function runCli(subArgs) {
  const argv = resolveCliArgv();
  let cmd, spawnArgs;
  if (argv.length === 2) {
    // [execPath, scriptPath] — invoke via Node
    cmd = argv[0];
    spawnArgs = [argv[1], ...subArgs];
  } else {
    // ['schemalint'] — rely on PATH
    cmd = argv[0];
    spawnArgs = subArgs;
  }
  return spawnSync(cmd, spawnArgs, { encoding: 'utf-8' });
}

function lint(schemaPath, options = {}) {
  const args = ['check', '--format', 'json'];
  if (options.profile) {
    if (Array.isArray(options.profile)) {
      options.profile.forEach((p) => args.push('--profile', p));
    } else {
      args.push('--profile', options.profile);
    }
  }
  args.push(schemaPath);

  const result = runCli(args);

  if (result.error && result.error.code === 'ENOENT') {
    throw new Error(
      'schemalint CLI not found on PATH. Install via: npm install -g @1nder-labs/cli'
    );
  }

  if (result.status !== 0 && result.status !== 1) {
    return { issues: [], error: result.stderr || 'schemalint process error' };
  }

  try {
    return JSON.parse(result.stdout || '{}');
  } catch {
    return { issues: [], error: result.stderr || 'failed to parse output' };
  }
}

function lintNode(projectPath, options = {}) {
  const args = ['check-node', '--format', 'json'];
  if (options.profile) {
    if (Array.isArray(options.profile)) {
      options.profile.forEach((p) => args.push('--profile', p));
    } else {
      args.push('--profile', options.profile);
    }
  }
  args.push('--source', projectPath);

  const result = runCli(args);

  if (result.error && result.error.code === 'ENOENT') {
    throw new Error(
      'schemalint CLI not found on PATH. Install via: npm install -g @1nder-labs/cli'
    );
  }

  if (result.status !== 0 && result.status !== 1) {
    return { issues: [], error: result.stderr || 'schemalint process error' };
  }

  try {
    return JSON.parse(result.stdout || '{}');
  } catch {
    return { issues: [], error: result.stderr || 'failed to parse output' };
  }
}

function lintPython(projectPath, options = {}) {
  const args = ['check-python', '--format', 'json'];
  if (options.profile) {
    if (Array.isArray(options.profile)) {
      options.profile.forEach((p) => args.push('--profile', p));
    } else {
      args.push('--profile', options.profile);
    }
  }
  args.push('--package', projectPath);

  const result = runCli(args);

  if (result.error && result.error.code === 'ENOENT') {
    throw new Error(
      'schemalint CLI not found on PATH. Install via: npm install -g @1nder-labs/cli'
    );
  }

  if (result.status !== 0 && result.status !== 1) {
    return { issues: [], error: result.stderr || 'schemalint process error' };
  }

  try {
    return JSON.parse(result.stdout || '{}');
  } catch {
    return { issues: [], error: result.stderr || 'failed to parse output' };
  }
}

module.exports = { lint, lintNode, lintPython };
