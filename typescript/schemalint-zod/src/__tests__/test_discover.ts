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
});
