import { afterAll, beforeAll, describe, expect, it, vi } from 'vitest';
import ts from 'typescript';
import { cpSync, mkdtempSync, realpathSync, rmSync, symlinkSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { discoverZodSchemas } from '../discover.js';

const here = path.dirname(fileURLToPath(import.meta.url));
const nodeModules = path.resolve(here, '../../node_modules');
const contracts = 'packages/shared/contracts/src';

/**
 * A monorepo copy under the temp dir, so no tsconfig.json sits above the
 * working directory, with node_modules linked in like a hoisted install.
 */
describe('discoverZodSchemas across a monorepo', () => {
  let originalCwd: string;
  let root: string;
  let link: string;

  beforeAll(() => {
    originalCwd = process.cwd();
    const tmp = mkdtempSync(path.join(realpathSync(tmpdir()), 'schemalint-mono-'));
    root = path.join(tmp, 'repo');
    cpSync(path.join(here, 'fixtures/monorepo'), root, { recursive: true });
    symlinkSync(nodeModules, path.join(root, 'node_modules'), 'dir');
    link = path.join(tmp, 'link');
    symlinkSync(root, link, 'dir');
    process.chdir(root);
  });

  afterAll(() => {
    process.chdir(originalCwd);
    rmSync(path.dirname(root), { recursive: true, force: true });
  });

  const names = (result: { models: { name: string }[] }): string[] =>
    result.models.map((m) => m.name).sort();

  it('builds a program from the package tsconfig when run from the repo root', async () => {
    const result = await discoverZodSchemas(contracts);

    expect(result.failures).toEqual([]);
    expect(names(result)).toEqual(['A', 'B1', 'B2', 'Wrapped']);
  });

  it('synthesizes a default program for files with no tsconfig above them', async () => {
    const result = await discoverZodSchemas('tools/loose.ts');

    expect(result.failures).toEqual([]);
    expect(names(result)).toEqual(['Loose']);
  });

  it('follows barrel re-exports to the real definition, once each', async () => {
    const barrel = await discoverZodSchemas(`${contracts}/index.ts`);

    expect(names(barrel)).toEqual(['A', 'B1', 'B2']);
    const b1 = barrel.models.find((m) => m.name === 'B1');
    expect(b1?.module_path).toMatch(/nested[\\/]b\.ts$/);
    expect(b1?.source_map['/properties/b'].file).toMatch(/nested[\\/]b\.ts$/);

    // Reached through the barrel and directly, but reported once.
    const both = await discoverZodSchemas(contracts);
    expect(both.models.filter((m) => m.name === 'B1')).toHaveLength(1);
    expect(both.models.filter((m) => m.name === 'A')).toHaveLength(1);
  });

  it('matches an absolute path through a symlinked directory', async () => {
    const result = await discoverZodSchemas(
      path.join(link, contracts, 'a.ts')
    );

    expect(result.warnings).toEqual([]);
    expect(names(result)).toEqual(['A']);
  });

  it('discovers an exported alias of a local z.object schema', async () => {
    const byFile = await discoverZodSchemas(`${contracts}/alias.ts`);

    expect(byFile.failures).toEqual([]);
    expect(names(byFile)).toEqual(['Wrapped']);
    expect(byFile.models[0].schema).toHaveProperty('properties.x');
    expect(byFile.models[0].source_map).toHaveProperty('/properties/x');

    const byDir = await discoverZodSchemas(contracts);
    expect(byDir.models.filter((m) => m.name === 'Wrapped')).toHaveLength(1);
  });

  it('walks a glob only from its static base', async () => {
    const walk = vi.spyOn(ts.sys, 'readDirectory');
    try {
      const result = await discoverZodSchemas('packages/shared/contracts/**/a.ts');
      expect(names(result)).toEqual(['A']);
      const roots = walk.mock.calls.map(([dir]) => dir);
      expect(roots.length).toBeGreaterThan(0);
      for (const dir of roots) {
        expect(dir).toContain(path.join('packages', 'shared', 'contracts'));
      }
    } finally {
      walk.mockRestore();
    }
  });

  it('reaches a sibling package through a ../ glob', async () => {
    process.chdir(path.join(root, 'packages/shared/contracts'));
    try {
      const result = await discoverZodSchemas('../other/**/*.ts');
      expect(names(result)).toEqual(['Other']);
    } finally {
      process.chdir(root);
    }
  });

  it('skips tsconfig-excluded files for dir and glob scopes but not a named file', async () => {
    const byDir = await discoverZodSchemas(contracts);
    const byGlob = await discoverZodSchemas(`${contracts}/**/*.ts`);
    expect(names(byDir)).not.toContain('Gen');
    expect(names(byGlob)).not.toContain('Gen');

    const named = await discoverZodSchemas(`${contracts}/generated/g.ts`);
    expect(names(named)).toEqual(['Gen']);
  });
});
