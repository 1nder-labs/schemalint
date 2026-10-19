import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';

import { discoverZodSchemas } from '../discover.js';

const fixturesDir = path.join(
  path.dirname(fileURLToPath(import.meta.url)),
  'fixtures'
);

describe('every Zod object schema a named file builds without run-time input', () => {
  let originalCwd: string;

  beforeAll(() => {
    originalCwd = process.cwd();
    process.chdir(fixturesDir);
  });

  afterAll(() => {
    process.chdir(originalCwd);
  });

  it('is linted, exported or not, declared or inline, in a function or not, however it is spelled', async () => {
    const result = await discoverZodSchemas('schema-roots.ts');

    expect(result.failures).toEqual([]);
    const fields = result.models
      .map((model) => Object.keys(model.schema.properties as object)[0])
      .sort();
    expect(fields).toEqual(['declared', 'exported', 'inFunction', 'inline', 'made', 'nested']);
    const outer = result.models.find((model) => model.name === 'Outer');
    expect(outer?.source_map).toHaveProperty('/properties/nested/properties/deep');
  });

  it('converts a zod/v4 schema when the zod package root is still v3', async () => {
    const result = await discoverZodSchemas('zod-v4-subpath.ts');

    expect(result.failures).toEqual([]);
    expect(result.models.map((model) => model.name)).toEqual(['ViaSubpath']);
  });
});
