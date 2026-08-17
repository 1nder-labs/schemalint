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

  // Documents the deferred half of KTD2: a property whose value is an
  // identifier reference to a separately declared `z.object()`, rather
  // than an inline literal, gets no source-map entry for anything inside
  // it. Resolving the identifier to its declaration is out of scope here —
  // the Rust-side ancestor walk (crates/schemalint/src/cli/pipeline/evaluate.rs)
  // covers the resulting gap by falling back to this outer entry. If this
  // test starts failing because `/properties/a/properties/site` gained a
  // map entry, identifier resolution has been implemented — update this
  // test deliberately rather than only making it pass.
  it('does not map inside a property whose value is an identifier, not an inline z.object() literal', async () => {
    const result = await discoverZodSchemas('nested-identifier.ts');

    expect(result.models).toHaveLength(1);
    const model = result.models[0];
    expect(model.name).toBe('Outer');

    expect(model.source_map).toHaveProperty('/properties/a');
    expect(Object.keys(model.source_map)).not.toContain(
      '/properties/a/properties/site'
    );
  });

  it('returns empty results for non-matching glob and names cause 1: no file on disk', async () => {
    const result = await discoverZodSchemas('nonexistent*.ts');

    expect(result.models).toHaveLength(0);
    expect(result.warnings).toHaveLength(1);
    expect(result.warnings[0].message).toContain('No file on disk matched');
    expect(result.warnings[0].message).toContain('nonexistent*.ts');
    expect(result.counts).toEqual({
      attempted: 0,
      excluded: 0,
      discovered: 0,
      failed: 0,
    });
  });

  it('names cause 2: files on disk but outside the TypeScript program', async () => {
    const result = await discoverZodSchemas('outside-include/*.ts');

    expect(result.models).toHaveLength(0);
    expect(result.warnings).toHaveLength(1);
    expect(result.warnings[0].message).toContain('1 file(s)');
    expect(result.warnings[0].message).toContain('outside-include/*.ts');
    expect(result.warnings[0].message).toContain('outside the TypeScript program');
    expect(result.warnings[0].message).toContain('include');
    expect(result.warnings[0].message).toContain('tsconfig.json');
    expect(result.counts).toEqual({
      attempted: 0,
      excluded: 0,
      discovered: 0,
      failed: 0,
    });
  });

  it('names cause 3: files checked but no schema found', async () => {
    const result = await discoverZodSchemas('no-schema-content.ts');

    expect(result.models).toHaveLength(0);
    expect(result.warnings).toHaveLength(1);
    expect(result.warnings[0].message).toContain('Checked 1 file(s)');
    expect(result.warnings[0].message).toContain('no-schema-content.ts');
    expect(result.warnings[0].message).toContain('found none');
    expect(result.counts).toEqual({
      attempted: 0,
      excluded: 0,
      discovered: 0,
      failed: 0,
    });
  });

  it('applies exclusions before schema evaluation, with no cause warning for ordinary exclusion', async () => {
    const result = await discoverZodSchemas('simple.ts', ['simple.ts']);

    expect(result.models).toHaveLength(0);
    expect(result.warnings).toHaveLength(0);
    expect(result.counts).toEqual({
      attempted: 0,
      excluded: 1,
      discovered: 0,
      failed: 0,
    });
  });

  it('a successful discovery reports no empty-discovery warning', async () => {
    const result = await discoverZodSchemas('simple.ts');

    expect(result.models).toHaveLength(1);
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
    expect(result.models).toHaveLength(4);
    expect(result.failures).toHaveLength(1);
    expect(result.failures[0]).toMatchObject({
      kind: 'metadata',
      target: 'ai.generateObject',
    });
    expect(result.failures[0].message).toContain('required schema metadata');
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
      expect.arrayContaining(['variable'])
    );
    expect(properties).not.toContain('conditional');
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

  it('canonicalizes current SDK aliases, namespaces, providers, and envelopes', async () => {
    const result = await discoverZodSchemas('sdk-adapters.ts');

    expect(result.failures).toEqual([]);
    expect(result.models).toHaveLength(5);
    expect(result.models.map((model) => model.canonical_kind)).toEqual([
      'openai.zodTextFormat',
      'anthropic.zodOutputFormat',
      'ai.Output.object',
      'ai.Output.array',
      'ai.dynamicTool',
    ]);
    expect(result.models.slice(0, 2).map((model) => model.provider)).toEqual([
      { certainty: 'definitive', provider: 'openai' },
      { certainty: 'definitive', provider: 'anthropic' },
    ]);
    expect(result.models.slice(2).map((model) => model.provider)).toEqual([
      { certainty: 'ambiguous' },
      { certainty: 'ambiguous' },
      { certainty: 'ambiguous' },
    ]);
    expect(result.models[0].envelope.name).toMatchObject({
      value: 'open_response',
      required: true,
    });
    expect(result.models[0].envelope.name.span.line).toBeGreaterThan(0);
    expect(result.models[2].envelope).toMatchObject({
      name: { value: 'object_result' },
      description: { value: 'one object' },
    });
    expect(result.models.every((model) => model.usage_span.line! > 0)).toBe(true);
  });

  it('reports unresolved required envelope metadata as a typed failure', async () => {
    const result = await discoverZodSchemas('unresolved-envelope.ts');

    expect(result.models).toEqual([]);
    expect(result.failures).toHaveLength(1);
    expect(result.failures[0]).toMatchObject({
      kind: 'metadata',
      target: 'openai.zodTextFormat',
    });
    expect(result.failures[0].message).toContain("required field 'name'");
  });

  it('retains provider ownership on each target', async () => {
    const result = await discoverZodSchemas('anthropic-only.ts');

    expect(result.models[0].provider).toEqual({
      certainty: 'definitive',
      provider: 'anthropic',
    });
  });

  it('marks legacy exported schemas as provider-ambiguous', async () => {
    const result = await discoverZodSchemas('simple.ts');

    expect(result.models[0].provider).toEqual({ certainty: 'ambiguous' });
  });

  it('never finalizes generic provider ownership per source partition', async () => {
    const partitioned = await discoverZodSchemas(
      'provider-partition-openai.ts'
    );
    const complete = await discoverZodSchemas('provider-partition-*.ts');

    const partitionedGeneric = partitioned.models.find(
      (model) => model.canonical_kind === 'ai.Output.object'
    );
    const completeGeneric = complete.models.find(
      (model) => model.canonical_kind === 'ai.Output.object'
    );
    expect(partitionedGeneric?.provider).toEqual({ certainty: 'ambiguous' });
    expect(completeGeneric?.provider).toEqual({ certainty: 'ambiguous' });
    expect(complete.models.map((model) => model.provider)).toEqual(
      expect.arrayContaining([
        { certainty: 'definitive', provider: 'openai' },
        { certainty: 'definitive', provider: 'anthropic' },
      ])
    );
  });

  it('rejects divergent conditional schema and required-name metadata', async () => {
    const result = await discoverZodSchemas('conditional-metadata.ts');

    expect(result.models).toHaveLength(2);
    expect(result.failures).toHaveLength(3);
    expect(result.failures.map((failure) => failure.target)).toEqual([
      'ai.generateObject',
      'openai.zodTextFormat',
      'openai.zodTextFormat',
    ]);
    expect(result.failures[0].message).toContain('required schema metadata');
    expect(result.failures[1].message).toContain('required schema metadata');
    expect(result.failures[2].message).toContain("required field 'name'");
    expect(result.counts).toMatchObject({
      attempted: 5,
      discovered: 2,
      failed: 3,
    });
    expect(result.models.map((model) => model.name)).toEqual([
      'generateObject:First',
      'zodTextFormat:same_name',
    ]);
  });

  it('counts an evaluation failure once', async () => {
    // Regression: `failures` used to alias `discoveryFailures`, so every
    // evaluation failure also incremented `attempted`, and the Rust caller
    // rejected the response with "invalid discovery counts".
    const result = await discoverZodSchemas('eval-throws.ts');

    expect(result.models).toEqual([]);
    expect(result.failures).toHaveLength(1);
    expect(result.counts).toMatchObject({
      attempted: 1,
      discovered: 0,
      failed: 1,
    });
  });

  it('fails closed when static aliases form a cycle', async () => {
    const result = await discoverZodSchemas('cyclic-metadata.ts');

    expect(result.models).toEqual([]);
    expect(result.failures).toHaveLength(1);
    expect(result.failures[0]).toMatchObject({
      kind: 'metadata',
      target: 'openai.zodTextFormat',
    });
    expect(result.failures[0].message).toContain('required schema metadata');
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

  it('escapes RFC 6901 special characters in property names so the pointer still matches source attribution', async () => {
    // Regression: a Zod property named with '/' or '~' must still receive
    // its source line. The pointer key must be RFC 6901-escaped ('~' -> '~0'
    // first, then '/' -> '~1') the same way the Rust normalizer escapes the
    // matching `/properties/{key}` join, so `source_map.get(&pointer)` keeps
    // agreeing between the two sides.
    const result = await discoverZodSchemas('escaped-key-props.ts');

    const model = result.models[0];
    expect(model).toBeDefined();

    // 'a/b' → /properties/a~1b
    expect(model.source_map).toHaveProperty('/properties/a~1b');
    const slashSpan = model.source_map['/properties/a~1b'];
    expect(slashSpan.file).toContain('escaped-key-props.ts');
    expect(slashSpan.line).toBeGreaterThan(0);

    // 'c~d' → /properties/c~0d
    expect(model.source_map).toHaveProperty('/properties/c~0d');
    const tildeSpan = model.source_map['/properties/c~0d'];
    expect(tildeSpan.file).toContain('escaped-key-props.ts');
    expect(tildeSpan.line).toBeGreaterThan(0);

    // The raw, unescaped names must NOT appear as pointer keys.
    expect(Object.keys(model.source_map)).not.toContain('/properties/a/b');
    expect(Object.keys(model.source_map)).not.toContain('/properties/c~d');
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
