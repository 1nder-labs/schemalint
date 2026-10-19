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

  const fields = (result: Awaited<ReturnType<typeof discoverZodSchemas>>) =>
    result.models.map((model) => Object.keys(model.schema.properties as object)).sort();

  it('follows a schema derived across files with no zod import in between', async () => {
    const result = await discoverZodSchemas('review-chain-*.ts');

    expect(result.failures).toEqual([]);
    expect(fields(result)).toEqual([['id'], ['id'], ['id', 'note']]);
  });

  it('reports an aliased schema once, from a facade alone or with its barrel', async () => {
    const facade = await discoverZodSchemas('review-alias-facade.ts');
    expect(facade.failures).toEqual([]);
    expect(fields(facade)).toEqual([['aliased']]);

    const all = await discoverZodSchemas('review-alias-*.ts');
    expect(all.failures).toEqual([]);
    expect(fields(all)).toEqual([['aliased']]);
  });

  it('still finds a buildable schema inside one that uses run-time input', async () => {
    const result = await discoverZodSchemas('review-dynamic-extend.ts');

    expect(result.failures).toEqual([]);
    expect(fields(result)).toEqual([['fixedPart']]);
  });

  it('keeps a block-scoped schema apart from a module-level one of the same name', async () => {
    const result = await discoverZodSchemas('review-block-scope.ts');

    expect(result.failures).toEqual([]);
    expect(fields(result)).toEqual([['blockScoped'], ['outer']]);
  });
});
