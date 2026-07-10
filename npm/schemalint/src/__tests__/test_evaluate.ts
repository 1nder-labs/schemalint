import { mkdtemp, mkdir, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';

import { afterEach, describe, expect, it } from 'vitest';

import { evaluateSchema } from '../evaluate.js';

const temporaryDirectories: string[] = [];

afterEach(async () => {
  await Promise.all(
    temporaryDirectories.splice(0).map((dir) =>
      rm(dir, { recursive: true, force: true })
    )
  );
});

describe('evaluateSchema', () => {
  it('uses the user project package-level converter for Zod v4 and Mini', async () => {
    const project = await mkdtemp(path.join(tmpdir(), 'schemalint-zod-v4-'));
    temporaryDirectories.push(project);
    const zodPackage = path.join(project, 'node_modules', 'zod');
    await mkdir(zodPackage, { recursive: true });
    await writeFile(
      path.join(zodPackage, 'package.json'),
      JSON.stringify({ name: 'zod', version: '4.0.1', main: 'index.cjs' })
    );
    await writeFile(
      path.join(zodPackage, 'index.cjs'),
      "exports.toJSONSchema = schema => ({ converted: schema._zod.variant });"
    );
    const source = path.join(project, 'schema.mjs');
    await writeFile(
      source,
      "export const MiniSchema = { _zod: { variant: 'mini' } };"
    );

    await expect(evaluateSchema(source, 'MiniSchema')).resolves.toEqual({
      converted: 'mini',
    });
  });

  it('rejects values that are not recognized Zod schemas', async () => {
    const project = await mkdtemp(path.join(tmpdir(), 'schemalint-not-zod-'));
    temporaryDirectories.push(project);
    const source = path.join(project, 'schema.mjs');
    await writeFile(source, 'export const Value = {};');

    await expect(evaluateSchema(source, 'Value')).rejects.toThrow(
      'not a recognized Zod v3/v4 schema'
    );
  });
});
