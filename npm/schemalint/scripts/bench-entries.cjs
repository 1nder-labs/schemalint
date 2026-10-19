#!/usr/bin/env node
'use strict';

// Pack this package against a locally built binary, install it with every
// package manager into throwaway prefixes, and report per-entry correctness and
// --version start-up time as JSON with pass/fail gates.
//
//   node scripts/bench-entries.cjs [--binary ../../target/release/schemalint] [--runs 7]
//
// Needs npm, pnpm, bun, and yarn 1 on PATH; a missing manager is reported, not
// fatal. Real global links are snapshotted first and restored if anything
// touches them; a guard npm answers `npm prefix -g` with a sentinel prefix.

const fs = require('fs');
const os = require('os');
const path = require('path');
const crypto = require('crypto');
const { spawnSync } = require('child_process');

const config = require('../launcher/config.cjs');

const PACKAGE_DIR = path.resolve(__dirname, '..');
const NAME = config.PACKAGE.name;
const VERSION = config.VERSION;
const args = process.argv.slice(2);
const option = (flag, fallback) => {
  const index = args.indexOf(flag);
  return index === -1 ? fallback : args[index + 1];
};
const BINARY = path.resolve(option('--binary', path.join(PACKAGE_DIR, '../../target/release/schemalint')));
const RUNS = Number(option('--runs', '7'));
const ROOT = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'schemalint-entries-')));

function run(command, argv, options = {}) {
  return spawnSync(command, argv, { encoding: 'utf8', ...options, env: { ...process.env, ...options.env } });
}

function linkState(file) {
  try {
    return fs.readlinkSync(file);
  } catch (error) {
    return error.code === 'EINVAL' ? 'FILE' : 'ABSENT';
  }
}

function realGlobalLinks() {
  const outside = { cwd: os.tmpdir() };
  const dirs = [
    path.join(run('npm', ['prefix', '-g'], outside).stdout.trim(), 'bin'),
    path.join(os.homedir(), '.bun', 'bin'),
    process.env.PNPM_HOME,
    process.env.PNPM_HOME && path.join(process.env.PNPM_HOME, 'bin'),
    run('yarn', ['global', 'bin'], outside).stdout.trim(),
  ].filter(Boolean);
  return Object.fromEntries(dirs.map((dir) => {
    const file = path.join(dir, 'schemalint');
    return [file, linkState(file)];
  }));
}

function restoreChangedLinks(snapshot) {
  const changed = [];
  for (const [file, want] of Object.entries(snapshot)) {
    const got = linkState(file);
    if (got === want) continue;
    changed.push({ file, want, got });
    if (want === 'FILE') continue; // cannot restore file contents; reported only
    fs.rmSync(file, { force: true });
    if (want !== 'ABSENT') fs.symlinkSync(want, file);
  }
  return changed;
}

function sha256(file) {
  return crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
}

// Package + manifest + pre-seeded verified cache for the local binary, so no
// install ever downloads; a cache miss fails the digest check instead.
function preparePackage() {
  const target = config.getTarget();
  const archiveName = config.getArchive(target);
  const stage = path.join(ROOT, 'stage');
  fs.mkdirSync(stage, { recursive: true });
  fs.copyFileSync(BINARY, path.join(stage, 'schemalint'));
  const archive = path.join(ROOT, archiveName);
  const tar = run('tar', ['-czf', archive, '-C', stage, 'schemalint']);
  if (tar.status !== 0) throw new Error(tar.stderr);

  const pkg = path.join(ROOT, 'pkg');
  fs.cpSync(PACKAGE_DIR, pkg, {
    recursive: true,
    filter: (source) => !source.includes(`${path.sep}node_modules`),
  });
  fs.writeFileSync(path.join(pkg, 'native-manifest.json'), JSON.stringify({
    schema: 1,
    package: NAME,
    version: VERSION,
    targets: { [target]: { archive: archiveName, sha256: sha256(archive) } },
  }));
  const pack = run('npm', ['pack', '--ignore-scripts', '--json', '--pack-destination', ROOT], { cwd: pkg });
  if (pack.status !== 0) throw new Error(pack.stderr);

  return {
    tarball: path.join(ROOT, JSON.parse(pack.stdout)[0].filename),
    manifest: JSON.parse(fs.readFileSync(path.join(pkg, 'native-manifest.json'), 'utf8')),
    archive,
  };
}

// Fill the cache through the launcher's own install path, as a first run would.
async function seedCache({ manifest, archive }) {
  const { ensureBinary } = require('../launcher/ensure-binary.cjs');
  await ensureBinary({
    manifest,
    binaryPath: path.join(ROOT, 'cache', VERSION, config.getTarget(), 'schemalint'),
    download: async (_url, destination) => fs.copyFileSync(archive, destination),
  });
}

function guardEnvironment() {
  const guard = path.join(ROOT, 'guard');
  const sentinel = path.join(ROOT, 'sentinel');
  fs.mkdirSync(guard);
  fs.mkdirSync(path.join(sentinel, 'bin'), { recursive: true });
  fs.symlinkSync('/sentinel-target', path.join(sentinel, 'bin', 'schemalint'));
  // The npm CLI beside this node, never a PATH lookup: a PATH shim (e.g. a
  // firewall wrapper) that re-resolves `npm` would loop back into this guard.
  const realNpm = fs.realpathSync(path.join(path.dirname(process.execPath), 'npm'));
  fs.writeFileSync(path.join(guard, 'npm'), [
    '#!/bin/sh',
    'depth=$(( ${SCHEMALINT_GUARD_DEPTH:-0} + 1 ))',
    '[ "$depth" -gt 8 ] && { echo "bench guard: npm re-entered $depth deep, aborting" >&2; exit 97; }',
    'export SCHEMALINT_GUARD_DEPTH=$depth',
    'if [ -z "${npm_config_prefix:-}" ]; then case "$*" in',
    `  "prefix -g"|"prefix --global"|"config get prefix") echo "${sentinel}"; exit 0;;`,
    `  "root -g") echo "${sentinel}/lib/node_modules"; exit 0;;`,
    'esac; fi',
    `exec "${process.execPath}" "${realNpm}" "$@"`,
    '',
  ].join('\n'), { mode: 0o755 });
  return { guard, sentinelLink: path.join(sentinel, 'bin', 'schemalint') };
}

function installs(tarball, fixture) {
  const prefix = (name) => path.join(ROOT, name);
  return [
    // First: it installs the zod that every entry's check-node run resolves.
    { name: 'npm local', cwd: fixture, argv: ['npm', 'install', '--no-audit', '--no-fund', tarball, 'zod@3'],
      entry: path.join(fixture, 'node_modules', '.bin', 'schemalint') },
    { name: 'npm -g', argv: ['npm', 'install', '-g', tarball], env: { npm_config_prefix: prefix('npm-g') },
      entry: path.join(prefix('npm-g'), 'bin', 'schemalint') },
    ...['blocked', 'allowed'].map((mode) => ({
      name: `pnpm -g ${mode}`,
      argv: ['pnpm', 'add', '-g', ...(mode === 'allowed' ? [`--allow-build=${NAME}`] : []), tarball],
      env: { PNPM_HOME: prefix(`pnpm-${mode}`) },
      path: path.join(prefix(`pnpm-${mode}`), 'bin'),
      entry: path.join(prefix(`pnpm-${mode}`), 'bin', 'schemalint'),
    })),
    ...['blocked', 'allowed'].map((mode) => ({
      name: `bun -g ${mode}`,
      argv: ['bun', 'add', '-g', ...(mode === 'allowed' ? ['--trust'] : []), tarball],
      env: { BUN_INSTALL: prefix(`bun-${mode}`), BUN_INSTALL_CACHE_DIR: prefix('bun-cache') },
      entry: path.join(prefix(`bun-${mode}`), 'bin', 'schemalint'),
    })),
    // ponytail: zod rides along because yarn 1 never installs peers; without it
    // the sidecar cannot resolve zod/v3 from the global folder (pre-existing).
    { name: 'yarn1 -g', argv: ['yarn', 'global', 'add', tarball, 'zod@3', '--prefix', prefix('yarn-g'),
      '--global-folder', prefix('yarn-gf'), '--cache-folder', prefix('yarn-cache')],
      entry: path.join(prefix('yarn-g'), 'bin', 'schemalint') },
  ];
}

function time(entry, env, cwd) {
  const once = () => {
    const started = process.hrtime.bigint();
    run(entry, ['--version'], { env, cwd });
    return Number(process.hrtime.bigint() - started) / 1e6;
  };
  once();
  const samples = Array.from({ length: RUNS }, once).sort((a, b) => a - b);
  return { min: +samples[0].toFixed(1), median: +samples[RUNS >> 1].toFixed(1) };
}

function entryKind(entry) {
  try {
    const head = Buffer.alloc(2);
    const fd = fs.openSync(fs.realpathSync(entry), 'r');
    fs.readSync(fd, head, 0, 2, 0);
    fs.closeSync(fd);
    return head.toString() === '#!' ? 'script' : 'native';
  } catch {
    return 'missing';
  }
}

async function main() {
  const realLinks = realGlobalLinks();
  const prepared = preparePackage();
  await seedCache(prepared);
  const { tarball } = prepared;
  const { guard, sentinelLink } = guardEnvironment();
  const fixture = path.join(ROOT, 'fixture');
  fs.mkdirSync(path.join(fixture, 'src'), { recursive: true });
  fs.writeFileSync(path.join(fixture, 'package.json'), '{"name":"fixture","private":true}\n');
  fs.writeFileSync(path.join(fixture, 'src', 'schema.ts'),
    "import { z } from 'zod';\nexport const Person = z.object({ name: z.string() });\n");
  const work = path.join(ROOT, 'work');
  fs.mkdirSync(work);

  const base = { PATH: `${guard}${path.delimiter}${process.env.PATH}`, SCHEMALINT_CACHE_DIR: path.join(ROOT, 'cache') };
  const results = [];
  const linkChanges = [];
  let sentinelIntact = true;
  for (const install of installs(tarball, fixture)) {
    const env = { ...base, ...install.env };
    if (install.path) env.PATH = `${install.path}${path.delimiter}${env.PATH}`;
    const result = run(install.argv[0], install.argv.slice(1), { cwd: install.cwd || work, env, timeout: 300_000 });
    linkChanges.push(...restoreChangedLinks(realLinks).map((change) => ({ ...change, after: install.name })));
    if (linkState(sentinelLink) !== '/sentinel-target') {
      sentinelIntact = false;
      fs.rmSync(sentinelLink, { force: true });
      fs.symlinkSync('/sentinel-target', sentinelLink);
    }
    const row = { name: install.name, installed: result.status === 0 };
    if (!row.installed) {
      row.error = (result.stderr || result.stdout || String(result.error)).trim().split('\n').slice(-3).join(' | ');
      results.push(row);
      continue;
    }
    row.kind = entryKind(install.entry);
    row.version = run(install.entry, ['--version'], { env }).stdout.trim();
    const helper = run(install.entry, ['check-node', '--profile', 'openai', '--source', 'src'], { env, cwd: fixture });
    row.helper_ok = helper.status === 0;
    if (!row.helper_ok) row.helper_error = (helper.stderr || helper.stdout).trim().split('\n').slice(0, 2).join(' | ');
    Object.assign(row, time(install.entry, env, fixture));
    results.push(row);
  }

  // A blocked install whose first-run download also fails must say how to fix it.
  const blocked = results.find((row) => row.name === 'pnpm -g blocked' && row.installed);
  let blockedErrorHelpful = null;
  if (blocked) {
    const notADirectory = path.join(ROOT, 'not-a-directory');
    fs.writeFileSync(notADirectory, '');
    const failure = run(path.join(ROOT, 'pnpm-blocked', 'bin', 'schemalint'), ['--version'],
      { env: { ...base, SCHEMALINT_CACHE_DIR: notADirectory } });
    blockedErrorHelpful = failure.status === 1 && /approve-builds/.test(failure.stderr) && /--trust/.test(failure.stderr);
  }

  const native = time(BINARY, base, fixture);
  const installed = results.filter((row) => row.installed);
  const report = {
    version: VERSION,
    root: ROOT,
    native,
    entries: results,
    gates: {
      all_entries_work: installed.length === results.length
        && installed.every((row) => row.version === `schemalint ${VERSION}` && row.helper_ok),
      sentinel_unchanged: sentinelIntact,
      real_links_unchanged: linkChanges.length === 0,
      blocked_error_helpful: blockedErrorHelpful,
    },
    worst_median_ms: Math.max(...installed.map((row) => row.median)),
    link_changes: linkChanges,
  };
  console.log(JSON.stringify(report, null, 2));
  process.exitCode = Object.values(report.gates).every((gate) => gate !== false) ? 0 : 1;
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
