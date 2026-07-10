#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const tarball = process.argv[2] && path.resolve(process.argv[2]);
const row = process.argv[3];
const schemalintBinary = process.argv[4] && path.resolve(process.argv[4]);

assert(tarball, 'usage: smoke-packed-sdk-consumers.cjs <package.tgz> <minimum|current> [schemalint]');
assert(['minimum', 'current'].includes(row), `unsupported SDK matrix row: ${row}`);
if (row === 'current') {
  assert(schemalintBinary, 'the current row requires a built schemalint binary');
}
const OPENAI = { certainty: 'definitive', provider: 'openai' };
const ANTHROPIC = { certainty: 'definitive', provider: 'anthropic' };
const AMBIGUOUS = { certainty: 'ambiguous' };
const NAME_64 = 'a'.repeat(64);
const NAME_65 = 'b'.repeat(65);

function field(required, value, lineMarker, token = `'${value}'`) {
  return { required, value, lineMarker, token };
}
function model(kind, provider, envelope = {}) {
  return { kind, provider, envelope };
}
const legacySource = [
  "import { z } from 'zod';",
  "import * as aiSdk from 'ai';",
  "import { Output as Result, dynamicTool as makeDynamicTool } from 'ai';",
  "import { zodResponseFormat, zodFunction as makeFunction } from 'openai/helpers/zod';",
  "import * as anthropicBeta from '@anthropic-ai/sdk/helpers/beta/zod';",
  '',
  'const Shared = z.object({ value: z.string() });',
  '',
  "aiSdk.Output.object({ name: 'legacy_object', description: 'legacy object', schema: Shared });",
  "Result.array({ name: 'legacy_array', element: z.object({ item: z.string() }) });",
  "makeDynamicTool({ description: 'legacy dynamic', inputSchema: Shared, execute: async () => ({}) });",
  "zodResponseFormat(Shared, 'legacy_response');",
  "makeFunction({ name: 'legacy_function', parameters: Shared });",
  "anthropicBeta.betaZodTool({ name: 'legacy_anthropic_tool', description: 'legacy tool', inputSchema: Shared, run: async () => 'ok' });",
  'const unresolvedName = String(Date.now());',
  'zodResponseFormat(Shared, unresolvedName);',
].join('\n');
const structuredFloorSource = [
  "import { z } from 'zod';",
  "import { Output } from 'ai';",
  "import { zodResponseFormat as responseAlias, zodTextFormat } from 'openai/helpers/zod';",
  "import * as openaiZod from 'openai/helpers/zod';",
  "import { zodOutputFormat as outputAlias } from '@anthropic-ai/sdk/helpers/zod';",
  "import * as anthropicZod from '@anthropic-ai/sdk/helpers/zod';",
  '',
  'const Shared = z.object({ value: z.string() });',
  '',
  "Output.object({ name: 'floor_object', schema: Shared });",
  "responseAlias(Shared, 'floor_response');",
  "zodTextFormat(Shared, 'floor_text');",
  "openaiZod.zodTextFormat(Shared, 'floor_namespace_text');",
  'outputAlias(Shared);',
  'anthropicZod.zodOutputFormat(Shared);',
].join('\n');
const currentPartialSource = [
  "import { z } from 'zod';",
  "import * as aiSdk from 'ai';",
  "import { Output as Result, dynamicTool as makeDynamicTool } from 'ai';",
  "import { zodResponseFormat, zodTextFormat as openaiText, zodFunction } from 'openai/helpers/zod';",
  "import * as anthropicZod from '@anthropic-ai/sdk/helpers/zod';",
  "import { betaZodTool as makeBetaTool } from '@anthropic-ai/sdk/helpers/beta/zod';",
  '',
  'const Clean = z.object({ value: z.string() });',
  '',
  "aiSdk.Output.object({ name: 'current_object', description: 'current object', schema: Clean });",
  "Result.array({ name: 'current_array', element: z.object({ item: z.string() }) });",
  "makeDynamicTool({ description: 'current dynamic', inputSchema: Clean, execute: async () => ({}) });",
  "zodResponseFormat(Clean, '');",
  `zodResponseFormat(Clean, '${NAME_64}');`,
  `zodResponseFormat(Clean, '${NAME_65}');`,
  "openaiText(Clean, 'bad name');",
  "zodFunction({ name: 'current_function', parameters: Clean });",
  "makeBetaTool({ name: 'current_anthropic_tool', description: 'current tool', inputSchema: Clean, run: async () => 'ok' });",
  'anthropicZod.zodOutputFormat(Clean);',
  'const unresolvedName = String(Date.now());',
  'openaiText(Clean, unresolvedName);',
].join('\n');
const currentCompleteSource = [
  "import { z } from 'zod';",
  "import { zodResponseFormat as openaiResponse } from 'openai/helpers/zod';",
  "import * as anthropicZod from '@anthropic-ai/sdk/helpers/zod';",
  '',
  'const SharedRestricted = z.object({ count: z.number().min(1) });',
  '',
  'anthropicZod.zodOutputFormat(SharedRestricted);',
  "openaiResponse(SharedRestricted, 'complete_openai');",
].join('\n');
const matrix = {
  minimum: [
    {
      label: 'legacy-floors',
      packages: ['ai@6.0.0', 'openai@4.55.0', '@anthropic-ai/sdk@0.63.0', 'zod@3.25.76'],
      fixtures: [{
        file: 'legacy.ts',
        source: legacySource,
        models: [
          model('ai.Output.object', AMBIGUOUS, {
            name: field(false, 'legacy_object', "aiSdk.Output.object({ name: 'legacy_object'"),
            description: field(false, 'legacy object', "description: 'legacy object'"),
          }),
          model('ai.Output.array', AMBIGUOUS, {
            name: field(false, 'legacy_array', "Result.array({ name: 'legacy_array'"),
          }),
          model('ai.dynamicTool', AMBIGUOUS, {
            description: field(false, 'legacy dynamic', "description: 'legacy dynamic'"),
          }),
          model('openai.zodResponseFormat', OPENAI, {
            name: field(true, 'legacy_response', "zodResponseFormat(Shared, 'legacy_response')"),
          }),
          model('openai.zodFunction', OPENAI, {
            name: field(true, 'legacy_function', "name: 'legacy_function'"),
          }),
          model('anthropic.betaZodTool', ANTHROPIC, {
            name: field(true, 'legacy_anthropic_tool', "name: 'legacy_anthropic_tool'"),
          }),
        ],
        failure: {
          target: 'openai.zodResponseFormat',
          lineMarker: 'zodResponseFormat(Shared, unresolvedName);',
          token: 'unresolvedName',
        },
      }],
    },
    {
      label: 'structured-output-floors',
      packages: ['ai@6.0.0', 'openai@4.87.0', '@anthropic-ai/sdk@0.72.0', 'zod@3.25.76'],
      fixtures: [{
        file: 'structured.ts',
        source: structuredFloorSource,
        models: [
          model('ai.Output.object', AMBIGUOUS, {
            name: field(false, 'floor_object', "Output.object({ name: 'floor_object'"),
          }),
          model('openai.zodResponseFormat', OPENAI, {
            name: field(true, 'floor_response', "responseAlias(Shared, 'floor_response')"),
          }),
          model('openai.zodTextFormat', OPENAI, {
            name: field(true, 'floor_text', "zodTextFormat(Shared, 'floor_text')"),
          }),
          model('openai.zodTextFormat', OPENAI, {
            name: field(true, 'floor_namespace_text', "openaiZod.zodTextFormat(Shared, 'floor_namespace_text')"),
          }),
          model('anthropic.zodOutputFormat', ANTHROPIC),
          model('anthropic.zodOutputFormat', ANTHROPIC),
        ],
      }],
    },
  ],
  current: [{
    label: 'current',
    packages: ['ai@7.0.19', 'openai@6.46.0', '@anthropic-ai/sdk@0.110.0', 'zod@4.4.3'],
    fixtures: [
      {
        file: 'partial.ts',
        source: currentPartialSource,
        models: [
          model('ai.Output.object', AMBIGUOUS, {
            name: field(false, 'current_object', "aiSdk.Output.object({ name: 'current_object'"),
            description: field(false, 'current object', "description: 'current object'"),
          }),
          model('ai.Output.array', AMBIGUOUS, {
            name: field(false, 'current_array', "Result.array({ name: 'current_array'"),
          }),
          model('ai.dynamicTool', AMBIGUOUS, {
            description: field(false, 'current dynamic', "description: 'current dynamic'"),
          }),
          model('openai.zodResponseFormat', OPENAI, {
            name: field(true, '', "zodResponseFormat(Clean, '');", "''"),
          }),
          model('openai.zodResponseFormat', OPENAI, {
            name: field(true, NAME_64, `zodResponseFormat(Clean, '${NAME_64}')`),
          }),
          model('openai.zodResponseFormat', OPENAI, {
            name: field(true, NAME_65, `zodResponseFormat(Clean, '${NAME_65}')`),
          }),
          model('openai.zodTextFormat', OPENAI, {
            name: field(true, 'bad name', "openaiText(Clean, 'bad name')"),
          }),
          model('openai.zodFunction', OPENAI, {
            name: field(true, 'current_function', "name: 'current_function'"),
          }),
          model('anthropic.betaZodTool', ANTHROPIC, {
            name: field(true, 'current_anthropic_tool', "name: 'current_anthropic_tool'"),
          }),
          model('anthropic.zodOutputFormat', ANTHROPIC),
        ],
        failure: {
          target: 'openai.zodTextFormat',
          lineMarker: 'openaiText(Clean, unresolvedName);',
          token: 'unresolvedName',
        },
        cli: {
          coverage: { status: 'partial', attempted: 11, excluded: 0, discovered: 10, checked: 7, failed: 4 },
          errors: 3,
          diagnostics: [
            { code: 'OAI-S-envelope-name', profile: 'openai.so.2026-04-30', pointer: '', lineMarker: "zodResponseFormat(Clean, '');", token: "''" },
            { code: 'OAI-S-envelope-name', profile: 'openai.so.2026-04-30', pointer: '', lineMarker: `zodResponseFormat(Clean, '${NAME_65}')`, token: `'${NAME_65}'` },
            { code: 'OAI-S-envelope-name', profile: 'openai.so.2026-04-30', pointer: '', lineMarker: "openaiText(Clean, 'bad name')", token: "'bad name'" },
          ],
          failureKinds: ['ai.Output.array', 'ai.Output.object', 'ai.dynamicTool', 'openai.zodTextFormat'],
        },
      },
      {
        file: 'complete.ts',
        source: currentCompleteSource,
        models: [
          model('anthropic.zodOutputFormat', ANTHROPIC),
          model('openai.zodResponseFormat', OPENAI, {
            name: field(true, 'complete_openai', "openaiResponse(SharedRestricted, 'complete_openai')"),
          }),
        ],
        cli: {
          coverage: { status: 'complete', attempted: 2, excluded: 0, discovered: 2, checked: 2, failed: 0 },
          errors: 1,
          diagnostics: [
            { code: 'ANT-K-minimum', profile: 'anthropic.so.2026-04-30', pointer: '/properties/count', lineMarker: 'const SharedRestricted', token: 'count', lineOnly: true },
          ],
          failureKinds: [],
        },
      },
    ],
  }],
};
function spanFor(source, filePath, lineMarker, token, lineOnly = false) {
  const lines = source.split('\n');
  const matching = lines.map((line, index) => ({ line, index }))
    .filter(({ line }) => line.includes(lineMarker));
  assert.equal(matching.length, 1, `expected one line containing ${lineMarker}`);
  const col = matching[0].line.indexOf(token);
  assert.notEqual(col, -1, `expected ${token} on line containing ${lineMarker}`);
  return {
    file: filePath.replaceAll('\\', '/'),
    line: matching[0].index + 1,
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
  const result = spawnSync(command, args, {
    encoding: 'utf8',
    timeout: 180_000,
    ...options,
  });
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
  const failureSpan = spanFor(fixture.source, filePath,
    fixture.failure.lineMarker, fixture.failure.token);
  assert.deepEqual(response.failures, [{
    kind: 'metadata',
    target: fixture.failure.target,
    message: `required field 'name' is not statically resolvable at ${failureSpan.file}:${failureSpan.line}:${failureSpan.col}`,
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
        target: 'ES2022',
        module: 'NodeNext',
        moduleResolution: 'NodeNext',
        skipLibCheck: true,
        strict: true,
      },
      include: ['*.ts'],
    }));
    for (const fixture of installation.fixtures) {
      fs.writeFileSync(path.join(root, fixture.file), `${fixture.source}\n`);
    }
    run(npm, [
      'install',
      '--ignore-scripts',
      '--no-audit',
      '--no-fund',
      '--package-lock=false',
      tarball,
      ...installation.packages,
    ], { cwd: root });
    const tsc = path.join(root, 'node_modules', '.bin', 'tsc');
    run(tsc, ['--noEmit'], { cwd: root });
    const helper = path.join(root, 'node_modules', '@1nder-labs', 'schemalint',
      'bin', 'schemalint-zod.js');
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
