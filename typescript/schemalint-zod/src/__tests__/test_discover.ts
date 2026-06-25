import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { discoverZodSchemas } from '../discover.js';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const fixturesDir = path.join(__dirname, 'fixtures');

describe('discoverZodSchemas', () => {
  let originalCwd: string;

  beforeAll(() => {
    originalCwd = process.cwd();
    process.chdir(fixturesDir);
  });

  afterAll(() => {
    process.chdir(originalCwd);
  });

  it('discovers a simple z.object schema with source map', async () => {
    const result = await discoverZodSchemas('simple.ts');

    expect(result.models).toHaveLength(1);
    const model = result.models[0];

    expect(model.name).toBe('UserSchema');
    expect(model.module_path).toContain('simple.ts');
    expect(model.schema).toHaveProperty('type', 'object');
    expect(model.schema).toHaveProperty('properties');

    // Verify source map has entries for all properties
    expect(model.source_map).toHaveProperty('/properties/email');
    expect(model.source_map).toHaveProperty('/properties/name');
    expect(model.source_map).toHaveProperty('/properties/age');

    const emailSpan = model.source_map['/properties/email'];
    expect(emailSpan.file).toContain('simple.ts');
    expect(emailSpan.line).toBeGreaterThan(0);
  });

  it('discovers multiple schemas in a single file', async () => {
    const result = await discoverZodSchemas('multiple.ts');

    expect(result.models).toHaveLength(2);

    const names = result.models.map((m) => m.name).sort();
    expect(names).toEqual(['Address', 'User']);

    for (const model of result.models) {
      expect(Object.keys(model.source_map).length).toBeGreaterThan(0);
    }
  });

  it('discovers nested z.object schemas with recursive source map', async () => {
    const result = await discoverZodSchemas('nested.ts');

    expect(result.models).toHaveLength(1);
    const model = result.models[0];

    expect(model.name).toBe('Order');

    expect(model.source_map).toHaveProperty('/properties/id');
    expect(model.source_map).toHaveProperty('/properties/customer');
    expect(model.source_map).toHaveProperty(
      '/properties/customer/properties/email'
    );
    expect(model.source_map).toHaveProperty(
      '/properties/customer/properties/address'
    );
    expect(model.source_map).toHaveProperty(
      '/properties/customer/properties/address/properties/street'
    );
    expect(model.source_map).toHaveProperty(
      '/properties/customer/properties/address/properties/city'
    );
  });

  it('returns empty results for non-matching glob', async () => {
    const result = await discoverZodSchemas('nonexistent*.ts');

    expect(result.models).toHaveLength(0);
    expect(result.warnings).toHaveLength(0);
  });

  it('produces 1-indexed source lines', async () => {
    const result = await discoverZodSchemas('simple.ts');
    const model = result.models[0];

    for (const span of Object.values(model.source_map)) {
      expect(span.line).toBeGreaterThanOrEqual(1);
    }
  });

  it('discovers provider-facing AI SDK call-site schemas', async () => {
    const result = await discoverZodSchemas('ai-sdk-calls.ts');

    expect(result.warnings).toHaveLength(0);
    expect(result.models).toHaveLength(5);
    expect(result.models.map((m) => m.name)).toEqual(
      expect.arrayContaining([
        'generateObject:LocalResult',
        'generateObject:VariableResult',
      ])
    );
    expect(result.models.some((m) => m.name.startsWith('streamObject:inline:')))
      .toBe(true);
    expect(result.models.some((m) => m.name.startsWith('tool:inline:')))
      .toBe(true);

    const properties = result.models.flatMap((m) =>
      Object.keys(m.schema.properties as Record<string, unknown>)
    );
    expect(properties).toEqual(
      expect.arrayContaining(['conditional', 'variable'])
    );
  });

  it('discovers schemas passed through provider helper factories', async () => {
    const result = await discoverZodSchemas('factory-calls.ts');

    expect(result.warnings).toHaveLength(0);
    expect(result.models).toHaveLength(2);
    expect(result.models.map((m) => m.name)).toEqual([
      'generateObject:extractThing',
      'generateObject:inlineThing',
    ]);

    const properties = result.models.map((m) =>
      Object.keys(m.schema.properties as Record<string, unknown>)
    );
    expect(properties).toEqual([['extracted'], ['inline']]);
  });

  it('discovers imported and tsconfig path-aliased schemas', async () => {
    const result = await discoverZodSchemas('imported-calls.ts');

    expect(result.warnings).toHaveLength(0);
    expect(result.models).toHaveLength(2);
    const properties = result.models.map((m) =>
      Object.keys(m.schema.properties as Record<string, unknown>)
    );
    expect(properties).toEqual([['imported'], ['aliased']]);
    expect(result.models[0].source_map).toHaveProperty('/properties/imported');
    expect(result.models[1].source_map).toHaveProperty('/properties/aliased');
  });

  it('discovers OpenAI and Anthropic helper schemas', async () => {
    const result = await discoverZodSchemas('provider-helpers.ts');

    expect(result.warnings).toHaveLength(0);
    expect(result.models).toHaveLength(3);
    expect(result.models.map((m) => m.name)).toEqual([
      'zodTextFormat:response',
      'zodFunction:lookup',
      'betaZodTool:search',
    ]);
  });

  it('sets provider_hint to "openai" when source imports from openai SDK', async () => {
    const result = await discoverZodSchemas('provider-helpers.ts');

    // provider-helpers.ts imports from 'openai/helpers/zod' (before @anthropic-ai/),
    // so the first-match wins and the hint should be "openai".
    expect(result.provider_hint).toBe('openai');
  });

  it('sets provider_hint to "anthropic" when source imports only from @anthropic-ai SDK', async () => {
    const result = await discoverZodSchemas('anthropic-only.ts');

    expect(result.provider_hint).toBe('anthropic');
  });

  it('leaves provider_hint undefined when source has no provider SDK imports', async () => {
    const result = await discoverZodSchemas('simple.ts');

    // simple.ts only imports from 'zod' — no provider SDK present.
    expect(result.provider_hint).toBeUndefined();
  });
});
