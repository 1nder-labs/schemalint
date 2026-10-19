import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { afterAll, beforeAll, describe, expect, it } from 'vitest';

import { discoverZodSchemas } from '../discover.js';

const fixturesDir = path.join(path.dirname(fileURLToPath(import.meta.url)), 'fixtures');

describe('batched evaluation per declaring file', () => {
  let originalCwd: string;
  beforeAll(() => {
    originalCwd = process.cwd();
    process.chdir(fixturesDir);
  });
  afterAll(() => process.chdir(originalCwd));

  it('evaluates every schema of a file from one module', async () => {
    const result = await discoverZodSchemas('batch-ok.ts');

    expect(result.failures).toEqual([]);
    expect(result.models.map((m) => [m.name, Object.keys(m.schema.properties as object)])).toEqual([
      ['First', ['a']],
      ['Second', ['b']],
    ]);
  });

  it('falls back per schema when one closure throws at import time', async () => {
    const result = await discoverZodSchemas('batch-poisoned.ts');

    expect(result.models.map((m) => m.name)).toEqual(['Good']);
    expect(Object.keys(result.models[0].schema.properties as object)).toEqual(['name']);
    expect(result.failures).toHaveLength(1);
    expect(result.failures[0].target).toBe('Broken');
    expect(result.failures[0].message).toContain('boom at import time');
    expect(result.counts).toMatchObject({ attempted: 2, discovered: 1, failed: 1 });
  });
});
