import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { discoverZodSchemas, toPosixPath } from '../discover.js';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const fixturesDir = path.join(__dirname, 'fixtures');

type Discovery = Awaited<ReturnType<typeof discoverZodSchemas>>;

/** The first property key of each model's schema, sorted: a stable handle on which schemas were found. */
function firstKeys(result: Discovery): string[] {
  return result.models
    .map((m) => Object.keys(m.schema.properties as Record<string, unknown>)[0])
    .sort();
}

describe('discoverZodSchemas', () => {
  let originalCwd: string;

  beforeAll(() => {
    originalCwd = process.cwd();
    process.chdir(fixturesDir);
  });

  afterAll(() => {
    process.chdir(originalCwd);
  });

  it('discovers multiple schemas in a single file', async () => {
    const result = await discoverZodSchemas('multiple.ts');

    expect(result.models.map((m) => m.name).sort()).toEqual(['Address', 'User']);
    for (const model of result.models) {
      expect(model.canonical_kind).toBe('zod.export');
      expect(model.provider).toEqual({ certainty: 'ambiguous' });
      expect(Object.keys(model.source_map).length).toBeGreaterThan(0);
    }
  });

  it('a successful discovery reports no warnings', async () => {
    const result = await discoverZodSchemas('simple.ts');

    expect(result.models).toHaveLength(1);
    expect(result.warnings).toHaveLength(0);
  });

  describe('empty discovery', () => {
    it('names cause 1: no file on disk', async () => {
      const result = await discoverZodSchemas('nonexistent*.ts');

      expect(result.models).toHaveLength(0);
      expect(result.warnings).toHaveLength(1);
      expect(result.warnings[0].message).toContain('No file on disk matched');
      expect(result.warnings[0].message).toContain('nonexistent*.ts');
      expect(result.counts).toEqual({ attempted: 0, excluded: 0, discovered: 0, failed: 0 });
    });

    it('checks files outside the tsconfig include list with a default program', async () => {
      const result = await discoverZodSchemas('outside-include/*.ts');

      expect(result.models).toHaveLength(0);
      expect(result.warnings).toHaveLength(1);
      expect(result.warnings[0].message).toContain('Checked 1 file(s)');
      expect(result.warnings[0].message).not.toContain('outside the TypeScript program');
    });

    it('names cause 3: files checked but no schema found', async () => {
      const result = await discoverZodSchemas('no-schema-content.ts');

      expect(result.models).toHaveLength(0);
      expect(result.warnings).toHaveLength(1);
      expect(result.warnings[0].message).toContain('Checked 1 file(s)');
      expect(result.warnings[0].message).toContain('no-schema-content.ts');
      expect(result.warnings[0].message).toContain('found none');
      expect(result.counts).toEqual({ attempted: 0, excluded: 0, discovered: 0, failed: 0 });
    });

    it('applies exclusions before evaluation, with no cause warning', async () => {
      const result = await discoverZodSchemas('simple.ts', ['simple.ts']);

      expect(result.models).toHaveLength(0);
      expect(result.warnings).toHaveLength(0);
      expect(result.counts).toEqual({ attempted: 0, excluded: 1, discovered: 0, failed: 0 });
    });
  });

  describe('scope', () => {
    it('lints a schema in a glob-scoped file that imports an SDK: there is no SDK gate', async () => {
      // sdk-untraced-call.ts imports 'ai'; sdk-untraced-export.ts holds a schema
      // that never reaches a provider call. Both are linted all the same.
      const result = await discoverZodSchemas('sdk-untraced-*.ts');

      expect(result.models.map((m) => m.name)).toEqual(['untracedSchema']);
      expect(result.failures).toEqual([]);
      expect(result.warnings).toEqual([]);
    });

    it('finds every module-level schema reachable from a call, with none reported twice', async () => {
      const result = await discoverZodSchemas('ai-sdk-calls.ts');

      expect(result.warnings).toHaveLength(0);
      expect(result.failures).toEqual([]);
      // Declared (LocalResult, VariableResult) and inline (conditional, chunk, query).
      expect(firstKeys(result)).toEqual(['chunk', 'conditional', 'query', 'title', 'variable']);
      expect(result.models.filter((m) => m.name === 'LocalResult')).toHaveLength(1);
      expect(result.models.some((m) => m.name.startsWith('inline:'))).toBe(true);
    });

    it('finds module-level schemas that wrappers and function-local aliases forward to a call', async () => {
      // A glob, not the file name: both scopes follow the same rule.
      const result = await discoverZodSchemas('wrapper-call*.ts');

      expect(result.warnings).toHaveLength(0);
      // `schema` parameters of the wrappers are runtime-dependent, so only the
      // module-scope declarations are roots.
      expect(firstKeys(result)).toEqual([
        'bare',
        'destructured',
        'direct',
        'injected',
        'renamed',
        'unrelated',
      ]);
      expect(result.models.map((m) => m.name)).toContain('unrelatedSchema');
    });

    it('a literal directory path lints every schema beneath it, each once', async () => {
      const result = await discoverZodSchemas('explicit-dir/');

      expect(result.models.map((m) => m.name).sort()).toEqual(['DeepSchema', 'TopSchema']);
      expect(result.warnings).toHaveLength(0);
    });

    it('resolves barrel re-exports, including tsconfig path aliases, to the declaring file', async () => {
      const result = await discoverZodSchemas('barrel.ts');

      expect(result.warnings).toHaveLength(0);
      expect(result.models.map((m) => m.name).sort()).toEqual(['AliasedSchema', 'ImportedSchema']);
      const byName = new Map(result.models.map((m) => [m.name, m]));
      expect(byName.get('ImportedSchema')!.module_path).toContain('schema-defs.ts');
      expect(byName.get('AliasedSchema')!.module_path).toContain('aliased-schema.ts');
      expect(byName.get('AliasedSchema')!.source_map).toHaveProperty('/properties/aliased');
    });

    it('does not report a schema twice when the barrel and its declaring file are both in scope', async () => {
      const result = await discoverZodSchemas('{barrel,schema-defs}.ts');

      expect(result.models.map((m) => m.name).sort()).toEqual(['AliasedSchema', 'ImportedSchema']);
    });

    it('matches only files under the working directory', async () => {
      const result = await discoverZodSchemas('simple.ts');

      for (const model of result.models) {
        expect(path.relative(fixturesDir, model.module_path)).not.toMatch(/^\.\./);
      }
    });
  });

  describe('failures and slicing', () => {
    it('counts an evaluation failure once', async () => {
      // Regression: every evaluation failure also incremented `attempted`, and
      // the Rust caller rejected the response with "invalid discovery counts".
      const result = await discoverZodSchemas('eval-throws.ts');

      expect(result.models).toEqual([]);
      expect(result.failures).toHaveLength(1);
      expect(result.failures[0].kind).toBe('evaluation');
      expect(result.counts).toMatchObject({ attempted: 1, discovered: 0, failed: 1 });
    });

    it('discovers an inline schema referencing a helper declared after it', async () => {
      // Regression: the synthetic module must include every module-level
      // declaration, not just those before the schema, or evaluation throws
      // a ReferenceError.
      const result = await discoverZodSchemas('forward-ref-helper.ts');

      expect(result.warnings).toHaveLength(0);
      expect(result.models).toHaveLength(1);
      expect(result.models[0].name).toMatch(/^inline:\d+$/);
      expect(firstKeys(result)).toEqual(['value']);
    });

    it('slices a noisy declaring file down to what the schema needs', async () => {
      // noisy-export.ts imports 'cloudflare:workflows' (rejected by Node's
      // default ESM loader) and throws at module scope. Neither is referenced
      // by NoisySchema, so slicing must drop both.
      const result = await discoverZodSchemas('noisy-export.ts');

      expect(result.warnings).toHaveLength(0);
      expect(result.failures).toEqual([]);
      expect(result.models.map((m) => m.name)).toEqual(['NoisySchema']);
      expect(firstKeys(result)).toEqual(['clean']);
    });

    it('slices a noisy file down to an inline schema and its forward-referenced helper', async () => {
      const result = await discoverZodSchemas('noisy-inline.ts');

      expect(result.warnings).toHaveLength(0);
      expect(result.failures).toEqual([]);
      expect(result.models).toHaveLength(1);
      expect(firstKeys(result)).toEqual(['value']);
    });
  });

  describe('toPosixPath (Windows path normalization)', () => {
    it('converts backslash-separated Windows paths to forward slashes', () => {
      // path.relative() on Windows returns "src\\models\\user.ts", which
      // picomatch fails to match against "src/**/*.ts".
      expect(toPosixPath('src\\models\\user.ts', '\\')).toBe('src/models/user.ts');
      expect(toPosixPath('..\\shared\\schema.ts', '\\')).toBe('../shared/schema.ts');
    });

    it('is a no-op for already-posix paths', () => {
      expect(toPosixPath('src/models/user.ts', '/')).toBe('src/models/user.ts');
    });

    it('a normalised Windows path matches a forward-slash picomatch glob', async () => {
      const { default: picomatch } = await import('picomatch');
      const isMatch = picomatch('src/**/*.ts', { dot: true });
      expect(isMatch(toPosixPath('src\\models\\user.ts', '\\'))).toBe(true);
      expect(isMatch('src\\models\\user.ts')).toBe(false);
    });
  });
});
