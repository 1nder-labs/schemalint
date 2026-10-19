import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { discoverZodSchemas } from '../discover.js';
import { escapePointerSegment } from '../discover_ast.js';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const fixturesDir = path.join(__dirname, 'fixtures');

describe('source maps', () => {
  let originalCwd: string;

  beforeAll(() => {
    originalCwd = process.cwd();
    process.chdir(fixturesDir);
  });

  afterAll(() => {
    process.chdir(originalCwd);
  });

  it('maps every property of a simple schema to a 1-indexed span', async () => {
    const result = await discoverZodSchemas('simple.ts');

    expect(result.models).toHaveLength(1);
    const model = result.models[0];
    expect(model.name).toBe('UserSchema');
    expect(model.module_path).toContain('simple.ts');
    expect(model.schema).toHaveProperty('type', 'object');

    for (const pointer of ['/properties/email', '/properties/name', '/properties/age']) {
      expect(model.source_map).toHaveProperty(pointer);
    }
    expect(model.source_map['/properties/email'].file).toContain('simple.ts');
    for (const span of Object.values(model.source_map)) {
      expect(span.line).toBeGreaterThanOrEqual(1);
    }
  });

  it('maps nested z.object schemas recursively', async () => {
    const result = await discoverZodSchemas('nested.ts');

    expect(result.models).toHaveLength(1);
    expect(result.models[0].name).toBe('Order');
    for (const pointer of [
      '/properties/customer/properties/email',
      '/properties/customer/properties/address/properties/street',
      '/properties/customer/properties/address/properties/city',
    ]) {
      expect(result.models[0].source_map).toHaveProperty(pointer);
    }
  });

  // The Rust-side ancestor walk covers this gap by falling back to the outer
  // entry. If `/properties/a/properties/site` gains a map entry, identifier
  // resolution has been implemented: update this test deliberately.
  it('does not map inside a property whose value is an identifier, not an inline z.object() literal', async () => {
    const result = await discoverZodSchemas('nested-identifier.ts');

    expect(result.models.map((m) => m.name).sort()).toEqual(['Inner', 'Outer']);
    const model = result.models.find((m) => m.name === 'Outer')!;
    expect(model.source_map).toHaveProperty('/properties/a');
    expect(Object.keys(model.source_map)).not.toContain('/properties/a/properties/site');
  });

  it('every model source_map carries a root entry pointing at the schema expression', async () => {
    const declared = await discoverZodSchemas('simple.ts');
    const inline = await discoverZodSchemas('forward-ref-helper.ts');

    for (const model of [declared.models[0], inline.models[0]]) {
      expect(model.source_map).toHaveProperty('');
      expect(model.source_map['']?.line).toBeGreaterThan(0);
    }
  });

  it('records spans for string-literal and computed-string-literal keys, not dynamic or spread ones', async () => {
    const model = (await discoverZodSchemas('string-key-props.ts')).models[0];

    expect(model).toBeDefined();
    expect(model.source_map['/properties/email'].file).toContain('string-key-props.ts');
    expect(model.source_map['/properties/email'].line).toBeGreaterThan(0);
    expect(model.source_map['/properties/name'].line).toBeGreaterThan(0);
    // No fabricated pointer for keys that cannot be resolved statically.
    expect(Object.keys(model.source_map)).not.toContain('/properties/dynamic');
    expect(Object.keys(model.source_map)).not.toContain('/properties/extra');
  });

  it('escapes RFC 6901 special characters so the pointer matches the Rust normalizer', async () => {
    const model = (await discoverZodSchemas('escaped-key-props.ts')).models[0];

    expect(model.source_map).toHaveProperty('/properties/a~1b');
    expect(model.source_map).toHaveProperty('/properties/c~0d');
    expect(model.source_map['/properties/a~1b'].file).toContain('escaped-key-props.ts');
    expect(Object.keys(model.source_map)).not.toContain('/properties/a/b');
    expect(Object.keys(model.source_map)).not.toContain('/properties/c~d');
  });
});

describe('pointer escaping', () => {
  // Checked against the table shared with the Rust normalizer and the Python
  // sidecar: `source_map.get(pointer)` joins them by exact string match.
  it('matches the cross-language contract', async () => {
    const fixturePath = path.resolve(
      __dirname,
      '../../../../crates/schemalint/tests/fixtures/pointer-escaping.json'
    );
    const { readFileSync } = await import('node:fs');
    const contract = JSON.parse(readFileSync(fixturePath, 'utf8')) as {
      cases: { input: string; escaped: string }[];
    };
    expect(contract.cases.length).toBeGreaterThan(0);

    for (const { input, escaped } of contract.cases) {
      expect(escapePointerSegment(input)).toBe(escaped);
    }
  });
});
