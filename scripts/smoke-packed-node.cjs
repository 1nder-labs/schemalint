'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const tarball = process.argv[2];
const zodVersion = process.argv[3] || '4.0.1';
const zodMode = process.argv[4] || 'full';
assert(
  tarball,
  'usage: smoke-packed-node.cjs <package.tgz> [zod-version] [full|mini]'
);
assert(['full', 'mini'].includes(zodMode), `unsupported Zod mode: ${zodMode}`);

const root = fs.mkdtempSync(path.join(os.tmpdir(), 'schemalint-packed-'));
try {
  fs.writeFileSync(
    path.join(root, 'package.json'),
    JSON.stringify({ private: true, type: 'module' })
  );
  fs.writeFileSync(
    path.join(root, 'tsconfig.json'),
    JSON.stringify({
      compilerOptions: {
        baseUrl: '.',
        paths: { '@schemas/*': ['schemas/*'] },
      },
    })
  );
  fs.mkdirSync(path.join(root, 'schemas'));
  fs.writeFileSync(
    path.join(root, 'schemas', 'definition.ts'),
    [
      `import { z } from '${zodMode === 'mini' ? 'zod/mini' : 'zod'}';`,
      'export const NameField = z.string();',
    ].join('\n')
  );
  fs.writeFileSync(
    path.join(root, 'schema.ts'),
    [
      `import { z } from '${zodMode === 'mini' ? 'zod/mini' : 'zod'}';`,
      "import { NameField } from '@schemas/definition';",
      'export const PackedSchema = z.object({ name: NameField });',
    ].join('\n')
  );

  const install = spawnSync(
    'npm',
    ['install', '--ignore-scripts', tarball, `zod@${zodVersion}`],
    { cwd: root, encoding: 'utf8' }
  );
  assert.equal(install.status, 0, install.stderr || install.stdout);

  const helper = path.join(
    root,
    'node_modules',
    '@1nder-labs',
    'schemalint',
    'bin',
    'schemalint-zod.js'
  );
  const input = [
    JSON.stringify({
      jsonrpc: '2.0',
      method: 'discover',
      params: { source: 'schema.ts' },
      id: 1,
    }),
    JSON.stringify({ jsonrpc: '2.0', method: 'shutdown', id: 2 }),
    '',
  ].join('\n');
  const smoke = spawnSync(process.execPath, [helper], {
    cwd: root,
    input,
    encoding: 'utf8',
    timeout: 30_000,
  });
  assert.equal(smoke.status, 0, smoke.stderr || smoke.stdout);

  const responses = smoke.stdout.trim().split('\n').map(JSON.parse);
  assert.equal(responses[0].result.counts.failed, 0);
  assert.equal(responses[0].result.models.length, 1);
  assert.equal(responses[0].result.models[0].name, 'PackedSchema');
  assert.equal(responses[1].result, 'ok');
  console.log(`packed Node runtime passed with Zod ${zodVersion} (${zodMode})`);
} finally {
  fs.rmSync(root, { recursive: true, force: true });
}
