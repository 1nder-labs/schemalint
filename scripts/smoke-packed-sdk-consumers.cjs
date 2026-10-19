#!/usr/bin/env node
'use strict';

// Installs the packed tarball next to real, pinned SDK versions and proves the
// sidecar finds exactly the Zod object schemas the fixture builds, by type.

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
if (row === 'current') assert(schemalintBinary, 'current row requires a schemalint binary');

// Oldest versions that ship every helper the fixture imports, and the latest
// verified ones. Minimum pairs the oldest SDKs with Zod 3; current uses Zod 4.
const PINS = {
  minimum: ['ai@6.0.0', 'openai@4.87.0', '@anthropic-ai/sdk@0.72.0', 'zod@3.25.76'],
  current: ['ai@7.0.19', 'openai@6.46.0', '@anthropic-ai/sdk@0.110.0', 'zod@4.4.3'],
};
const zodV4 = row === 'current';

// Each schema's first property key is its identity in the assertions below.
const FIXTURE = `import { z } from 'zod';
import { generateObject, Output, tool } from 'ai';
import { zodFunction, zodResponseFormat, zodTextFormat } from 'openai/helpers/zod';
import { zodOutputFormat } from '@anthropic-ai/sdk/helpers/zod';
import { betaZodTool } from '@anthropic-ai/sdk/helpers/beta/zod';

const AiConst = z.object({ ai_const: z.string() });
export const aiOutput = Output.object({ schema: AiConst });

export async function aiInlineInFunction(model: any) {
  return generateObject({ model, schema: z.object({ ai_in_function: z.string() }), prompt: 'x' });
}

export const aiTool = tool({
  description: 'x',
  inputSchema: z.object({ ai_tool: z.string() }),
  execute: async () => 'ok',
});

export const openaiResponse = zodResponseFormat(z.object({ oa_response: z.string() }), 'r');
const OaText = z.object({ oa_text: z.string() });
export const openaiText = zodTextFormat(OaText, 't');
export const openaiFunction = zodFunction({ name: 'f', parameters: z.object({ oa_function: z.string() }) });

export const anthropicOutput = zodOutputFormat(z.object({ an_output: z.string() }));
export const anthropicTool = betaZodTool({
  name: 't',
  description: 'x',
  inputSchema: z.object({ an_tool: z.string() }),
  run: async () => 'ok',
});

// Built from a function parameter: nothing to evaluate without running it.
export function fromParameter(shape: z.ZodRawShape) {
  return zodOutputFormat(z.object(shape));
}

// Zod 4 cannot express a Date in JSON Schema; Zod 3 renders it as a string.
export const WithDate = z.object({ date_field: z.date() });
`;
const EXPECTED_KEYS = [
  'ai_const', 'ai_in_function', 'ai_tool', 'oa_response', 'oa_text', 'oa_function',
  'an_output', 'an_tool', ...(zodV4 ? [] : ['date_field']),
];

function run(command, args, options) {
  const result = spawnSync(command, args, { encoding: 'utf8', timeout: 180_000, ...options });
  const output = `${result.stdout || ''}${result.stderr || ''}`;
  assert.equal(result.status, 0, `${command} ${args.join(' ')} failed\n${output}`);
  return result;
}

function discover(root, helper) {
  const input = [
    JSON.stringify({ jsonrpc: '2.0', method: 'discover', params: { source: 'sdk.ts' }, id: 1 }),
    JSON.stringify({ jsonrpc: '2.0', method: 'shutdown', id: 2 }),
    '',
  ].join('\n');
  const result = run(process.execPath, [helper], { cwd: root, input, timeout: 60_000 });
  const responses = result.stdout.trim().split('\n').map(JSON.parse);
  assert.equal(responses.length, 2);
  assert.equal(responses[1].result, 'ok');
  return responses[0].result;
}

function assertDiscovery(response) {
  const keys = response.models.map((m) => Object.keys(m.schema.properties || {})[0]);
  assert.deepEqual([...keys].sort(), [...EXPECTED_KEYS].sort(), 'discovered schemas differ');
  for (const model of response.models) {
    assert.equal(model.canonical_kind, 'zod.export');
    assert.deepEqual(model.provider, { certainty: 'ambiguous' });
    assert.deepEqual(model.envelope, {});
  }
  assert.deepEqual(response.failures, []);
  assert.deepEqual(response.counts, {
    attempted: EXPECTED_KEYS.length, excluded: 0, discovered: EXPECTED_KEYS.length, failed: 0,
  });
  const skipped = response.warnings.map((w) => w.message);
  if (zodV4) {
    assert.equal(skipped.length, 1, `expected one Skipped warning: ${skipped}`);
    assert(skipped[0].startsWith('Skipped:'), skipped[0]);
    assert.equal(response.warnings[0].model, 'WithDate');
  } else {
    assert.deepEqual(skipped, []);
  }
}

function assertCli(root, helper) {
  // package.json lists both SDK families, which selects both profiles.
  const result = spawnSync(schemalintBinary, ['check-node', '-S', 'sdk.ts', '-f', 'json'], {
    cwd: root, encoding: 'utf8', timeout: 60_000,
    env: { ...process.env, SCHEMALINT_ZOD_HELPER: helper },
  });
  const output = JSON.parse(result.stdout);
  const profiles = JSON.stringify(output.profiles);
  assert(/openai/i.test(profiles) && /anthropic/i.test(profiles), `profiles: ${profiles}`);
  const failures = (output.report && output.report.failures) || [];
  assert.deepEqual(
    failures.filter((f) => /ambiguous/i.test(`${f.kind} ${f.message}`)), [],
    'a target failed as provider-ambiguous'
  );
}

const npm = process.platform === 'win32' ? 'npm.cmd' : 'npm';
const root = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), `schemalint-sdk-${row}-`)));
try {
  fs.writeFileSync(path.join(root, 'package.json'), JSON.stringify({
    private: true, type: 'module',
    dependencies: { openai: '*', '@anthropic-ai/sdk': '*' },
  }));
  fs.writeFileSync(path.join(root, 'tsconfig.json'), JSON.stringify({
    compilerOptions: {
      target: 'ES2022', module: 'NodeNext', moduleResolution: 'NodeNext',
      skipLibCheck: true, strict: true,
    },
    include: ['*.ts'],
  }));
  fs.writeFileSync(path.join(root, 'sdk.ts'), FIXTURE);
  run(npm, [
    'install', '--ignore-scripts', '--no-audit', '--no-fund', '--package-lock=false',
    tarball, ...PINS[row],
  ], { cwd: root });
  run(path.join(root, 'node_modules', '.bin', 'tsc'), ['--noEmit'], { cwd: root });
  const helper = path.join(
    root, 'node_modules', '@1nder-labs', 'schemalint', 'bin', 'schemalint-zod.js'
  );
  assert(fs.existsSync(helper), 'packed sidecar is missing from installed package');
  assertDiscovery(discover(root, helper));
  if (schemalintBinary) assertCli(root, helper);
  console.log(`installed SDK consumer passed: ${row}`);
} finally {
  fs.rmSync(root, { recursive: true, force: true });
}
