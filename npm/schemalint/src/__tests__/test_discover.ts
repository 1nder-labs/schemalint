import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { discoverZodSchemas, toPosixPath } from '../discover.js';
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

  it('discovers inline schema referencing a helper declared after the call site', async () => {
    // Regression: the synthetic module must include all module-level
    // declarations, not just the ones that appear before the target expression.
    // Previously, `makeField()` — declared after the call site — was omitted,
    // causing a ReferenceError during evaluation.
    const result = await discoverZodSchemas('forward-ref-helper.ts');

    expect(result.warnings).toHaveLength(0);
    expect(result.models).toHaveLength(1);
    expect(result.models[0].schema).toHaveProperty('type', 'object');
    expect(result.models[0].schema).toHaveProperty('properties');
    const props = result.models[0].schema.properties as Record<string, unknown>;
    expect(Object.keys(props)).toContain('value');
  });

  it('buildSourceMapFromObjectLiteral records spans for string-literal and computed-string-literal property names', async () => {
    // Regression: the source-map builder previously skipped any property whose
    // name was not a bare Identifier.  String-literal keys ('email': ...) and
    // computed keys with a string-literal expression (['name']: ...) ARE
    // statically resolvable and must produce a /properties/<name> entry so that
    // diagnostics for those fields retain their source location.
    // Dynamic-computed ([k]: ...) and spread (...base) are NOT resolvable and
    // must NOT produce an entry (no fabricated pointer that matches nothing).
    const result = await discoverZodSchemas('string-key-props.ts');

    const model = result.models[0];
    expect(model).toBeDefined();

    // String-literal key: { 'email': z.string() } → must have span
    expect(model.source_map).toHaveProperty('/properties/email');
    const emailSpan = model.source_map['/properties/email'];
    expect(emailSpan.file).toContain('string-key-props.ts');
    expect(emailSpan.line).toBeGreaterThan(0);

    // Computed key with string-literal: { ['name']: z.string() } → must have span
    expect(model.source_map).toHaveProperty('/properties/name');
    const nameSpan = model.source_map['/properties/name'];
    expect(nameSpan.file).toContain('string-key-props.ts');
    expect(nameSpan.line).toBeGreaterThan(0);

    // Dynamic-computed ([k]: ...) → must NOT produce a pointer
    expect(Object.keys(model.source_map)).not.toContain('/properties/dynamic');

    // Spread (...base) → must NOT produce a pointer
    expect(Object.keys(model.source_map)).not.toContain('/properties/extra');
  });

  it('source glob filter does not drop files whose path shares a prefix with cwd but is outside it', async () => {
    // Regression: the old startsWith(projectRoot) check incorrectly accepted
    // a file at "/path/to/appExtra/foo.ts" when cwd is "/path/to/app", because
    // the string "/path/to/appExtra/foo.ts" starts with "/path/to/app".
    // path.relative() is correct: it only strips the prefix when the file is
    // genuinely under cwd.
    //
    // We verify the fix indirectly by confirming that a glob like "simple.ts"
    // matches exactly "simple.ts" (under cwd) and not a sibling directory whose
    // name extends the cwd basename — e.g., a hypothetical "fixtures-extra/simple.ts"
    // would have relPath "../../fixtures-extra/simple.ts" and must not match "simple.ts".
    const result = await discoverZodSchemas('simple.ts');
    // All matched files must live directly inside the fixtures dir (no path traversal).
    for (const model of result.models) {
      expect(model.module_path).toContain(fixturesDir);
      // The module path must NOT contain any path that traverses outside cwd.
      expect(path.relative(fixturesDir, model.module_path)).not.toMatch(/^\.\./);
    }
  });

  describe('toPosixPath (Windows path normalization)', () => {
    it('converts backslash-separated Windows paths to forward slashes', () => {
      // Simulate a Windows path.relative() output with '\\' as the separator.
      // On POSIX, path.sep === '/' so we pass '\\' explicitly to exercise the
      // Windows branch.  This is the regression guard: on Windows, path.relative()
      // returns e.g. "src\\models\\user.ts" which picomatch would fail to match
      // against "src/**/*.ts" — toPosixPath must convert it first.
      expect(toPosixPath('src\\models\\user.ts', '\\')).toBe('src/models/user.ts');
      expect(toPosixPath('src\\foo.ts', '\\')).toBe('src/foo.ts');
      expect(toPosixPath('..\\shared\\schema.ts', '\\')).toBe('../shared/schema.ts');
    });

    it('is a no-op for already-posix paths', () => {
      // On POSIX, sep === '/' so nothing changes.
      expect(toPosixPath('src/models/user.ts', '/')).toBe('src/models/user.ts');
      expect(toPosixPath('simple.ts', '/')).toBe('simple.ts');
    });

    it('normalised Windows path matches a forward-slash picomatch glob', async () => {
      // End-to-end proof: a backslash-style relPath, once normalised, must
      // satisfy a forward-slash glob — the exact condition that was broken on Windows.
      const { default: picomatch } = await import('picomatch');
      const isMatch = picomatch('src/**/*.ts', { dot: true });
      expect(isMatch(toPosixPath('src\\models\\user.ts', '\\'))).toBe(true);
      // Without normalisation the match would fail:
      expect(isMatch('src\\models\\user.ts')).toBe(false);
    });
  });
});
