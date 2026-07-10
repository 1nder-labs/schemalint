#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const { matrix } = require('./packed-sdk-fixtures.cjs');

const tarball = process.argv[2] && path.resolve(process.argv[2]);
const row = process.argv[3];
const schemalintBinary = process.argv[4] && path.resolve(process.argv[4]);
assert(tarball, 'usage: smoke-packed-sdk-consumers.cjs <package.tgz> <minimum|current> [schemalint]');
assert(['minimum', 'current'].includes(row), `unsupported SDK matrix row: ${row}`);
if (row === 'current') assert(schemalintBinary, 'current row requires a schemalint binary');

function spanFor(source, filePath, marker, token, lineOnly = false) {
  const matches = source.split('\n').map((line, index) => ({ line, index }))
    .filter(({ line }) => line.includes(marker));
  assert.equal(matches.length, 1, `expected one line containing ${marker}`);
  const col = matches[0].line.indexOf(token);
  assert.notEqual(col, -1, `expected ${token} on line containing ${marker}`);
  return {
    file: filePath.replaceAll('\\', '/'),
    line: matches[0].index + 1,
    ...(lineOnly ? {} : { col: col + 1 }),
  };
}

function expectedEnvelope(source, filePath, fields) {
  return Object.fromEntries(Object.entries(fields).map(([name, spec]) => [name, {
    required: spec.required,
    span: spanFor(source, filePath, spec.lineMarker, spec.token),
    value: spec.value,
  }]));
}

function run(command, args, options) {
  const result = spawnSync(command, args, { encoding: 'utf8', timeout: 180_000, ...options });
  const output = `${result.stdout || ''}${result.stderr || ''}`;
  assert.equal(result.status, 0, `${command} ${args.join(' ')} failed\n${output}`);
  return result;
}

function runSidecar(root, helper, fixture) {
  const input = [
    JSON.stringify({ jsonrpc: '2.0', method: 'discover', params: { source: fixture.file }, id: 1 }),
    JSON.stringify({ jsonrpc: '2.0', method: 'shutdown', id: 2 }),
    '',
  ].join('\n');
  const result = run(process.execPath, [helper], { cwd: root, input, timeout: 60_000 });
  const responses = result.stdout.trim().split('\n').map(JSON.parse);
  assert.equal(responses.length, 2);
  assert.equal(responses[1].result, 'ok');
  return responses[0].result;
}

function assertSidecar(root, fixture, response) {
  const filePath = path.join(root, fixture.file);
  const expectedFailed = fixture.failure ? 1 : 0;
  assert.deepEqual(response.counts, {
    attempted: fixture.models.length + expectedFailed,
    excluded: 0,
    discovered: fixture.models.length,
    failed: expectedFailed,
  });
  assert.deepEqual(response.warnings, []);
  assert.equal(response.models.length, fixture.models.length);
  response.models.forEach((actual, index) => {
    const expected = fixture.models[index];
    assert.equal(actual.canonical_kind, expected.kind, `${fixture.file} model ${index}`);
    assert.deepEqual(actual.provider, expected.provider, `${fixture.file} provider ${index}`);
    assert.deepEqual(
      actual.envelope,
      expectedEnvelope(fixture.source, filePath, expected.envelope),
      `${fixture.file} envelope ${index}`
    );
  });
  if (!fixture.failure) {
    assert.deepEqual(response.failures, []);
    return;
  }
  const failureSpan = spanFor(
    fixture.source, filePath, fixture.failure.lineMarker, fixture.failure.token
  );
  assert.deepEqual(response.failures, [{
    kind: 'metadata',
    target: fixture.failure.target,
    message: `${fixture.failure.message || "required field 'name'"} is not statically resolvable at ${failureSpan.file}:${failureSpan.line}:${failureSpan.col}`,
  }]);
}

function assertCli(root, helper, fixture) {
  const result = spawnSync(
    schemalintBinary,
    ['check-node', '--source', fixture.file, '--format', 'json'],
    {
      cwd: root,
      encoding: 'utf8',
      timeout: 60_000,
      env: { ...process.env, SCHEMALINT_ZOD_HELPER: helper },
    }
  );
  assert.equal(result.status, 1, result.stdout + result.stderr);
  const output = JSON.parse(result.stdout);
  assert.deepEqual(output.report.coverage, fixture.cli.coverage);
  assert.equal(output.report.success, false);
  assert.equal(output.summary.errors, fixture.cli.errors);
  assert.equal(output.summary.schemas_checked, fixture.cli.coverage.checked);
  const expectedDiagnostics = fixture.cli.diagnostics.map((diagnostic) => ({
    code: diagnostic.code,
    profile: diagnostic.profile,
    pointer: diagnostic.pointer,
    source: spanFor(
      fixture.source,
      path.join(root, fixture.file),
      diagnostic.lineMarker,
      diagnostic.token,
      diagnostic.lineOnly
    ),
  }));
  const actualDiagnostics = output.diagnostics.map(
    ({ code, profile, pointer, source }) => ({ code, profile, pointer, source })
  );
  const byLocation = (left, right) =>
    `${left.code}:${left.source.line}:${left.source.col || 0}`.localeCompare(
      `${right.code}:${right.source.line}:${right.source.col || 0}`
    );
  assert.deepEqual(actualDiagnostics.sort(byLocation), expectedDiagnostics.sort(byLocation));
  const failureText = output.report.failures
    .map((failure) => `${failure.target}: ${failure.message}`)
    .join('\n');
  assert.equal(output.report.failures.length, fixture.cli.failureKinds.length);
  for (const kind of fixture.cli.failureKinds) {
    assert(failureText.includes(kind), `missing retained failure for ${kind}`);
  }
}

const npm = process.platform === 'win32' ? 'npm.cmd' : 'npm';
for (const installation of matrix[row]) {
  const root = fs.realpathSync(
    fs.mkdtempSync(path.join(os.tmpdir(), `schemalint-sdk-${installation.label}-`))
  );
  try {
    fs.writeFileSync(path.join(root, 'package.json'), JSON.stringify({ private: true, type: 'module' }));
    fs.writeFileSync(path.join(root, 'tsconfig.json'), JSON.stringify({
      compilerOptions: {
        target: 'ES2022', module: 'NodeNext', moduleResolution: 'NodeNext',
        skipLibCheck: true, strict: true,
      },
      include: ['*.ts'],
    }));
    for (const fixture of installation.fixtures) {
      fs.writeFileSync(path.join(root, fixture.file), `${fixture.source}\n`);
    }
    run(npm, [
      'install', '--ignore-scripts', '--no-audit', '--no-fund', '--package-lock=false',
      tarball, ...installation.packages,
    ], { cwd: root });
    const tsc = path.join(root, 'node_modules', '.bin', 'tsc');
    run(tsc, ['--noEmit'], { cwd: root });
    const helper = path.join(
      root, 'node_modules', '@1nder-labs', 'schemalint', 'bin', 'schemalint-zod.js'
    );
    assert(fs.existsSync(helper), 'packed sidecar is missing from installed package');
    for (const fixture of installation.fixtures) {
      const response = runSidecar(root, helper, fixture);
      assertSidecar(root, fixture, response);
      if (fixture.cli) assertCli(root, helper, fixture);
    }
    console.log(`installed SDK consumer passed: ${installation.label}`);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
}
