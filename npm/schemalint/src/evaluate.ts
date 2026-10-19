/**
 * Runtime schema evaluation.
 *
 * Given a file path and export name, dynamically imports the user's TypeScript
 * file, accesses the exported Zod schema, and converts it to JSON Schema.
 *
 * Uses Zod v4's package-level `toJSONSchema()` API for v4 and Mini schemas,
 * and `zod-to-json-schema` for Zod v3.
 */

import { randomUUID } from 'node:crypto';
import { existsSync } from 'node:fs';
import { createRequire } from 'node:module';
import { mkdir, mkdtemp, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { pathToFileURL } from 'node:url';

const localRequire = createRequire(import.meta.url);

/**
 * Dynamically import a user's TypeScript file and evaluate an exported
 * Zod schema to JSON Schema.
 *
 * Requires `tsx` for JIT compilation of TypeScript imports.
 */
export async function evaluateSchema(
  filePath: string,
  exportName: string
): Promise<Record<string, unknown>> {
  return withStdoutRedirect(async () => {
    return convertExport(await importModule(filePath), exportName, filePath);
  });
}

function convertExport(
  mod: Record<string, unknown>,
  exportName: string,
  filePath: string
): Record<string, unknown> {
  const schema = mod[exportName];
  if (!schema) {
    throw new Error(
      `Export '${exportName}' not found in module '${filePath}'. ` +
        `Available exports: ${Object.keys(mod).join(', ') || '(none)'}`
    );
  }
  return zodToJsonSchema(schema, filePath);
}

/**
 * One temp root for a whole run. Each distinct `node_modules` is symlinked
 * once into its own subdirectory, so synthetic modules written there resolve
 * packages the way the declaring file would.
 */
export interface EvalWorkspace {
  /** A fresh `.ts` path in a directory that resolves `baseFilePath`'s packages. */
  newModulePath(baseFilePath: string): Promise<string>;
  dispose(): Promise<void>;
}

export async function createEvalWorkspace(): Promise<EvalWorkspace> {
  const root = await mkdtemp(join(tmpdir(), 'schemalint-zod-'));
  const dirs = new Map<string, Promise<string>>();
  return {
    newModulePath: async (baseFilePath) => {
      const nodeModules = findNearestNodeModules(baseFilePath) ?? '';
      let dir = dirs.get(nodeModules);
      if (!dir) {
        const target = join(root, String(dirs.size));
        dir = mkdir(target).then(async () => {
          if (nodeModules) await linkNodeModules(target, nodeModules);
          return target;
        });
        dirs.set(nodeModules, dir);
      }
      return join(await dir, `${randomUUID()}.ts`);
    },
    dispose: () => rm(root, { recursive: true, force: true }),
  };
}

export type ExportOutcome =
  | { ok: true; schema: Record<string, unknown> }
  | { ok: false; error: unknown };

/**
 * Evaluate several exports of one synthetic module with a single write and
 * import. Each export converts on its own, so one conversion error stays with
 * its export. A throw while importing the module affects every export, so it
 * rejects instead; callers retry the exports individually.
 */
export async function evaluateSyntheticExports(
  source: string,
  exportNames: readonly string[],
  baseFilePath: string,
  workspace: EvalWorkspace
): Promise<ExportOutcome[]> {
  const filePath = await workspace.newModulePath(baseFilePath);
  await writeFile(filePath, source, 'utf8');
  return withStdoutRedirect(async () => {
    const mod = await importModule(filePath);
    return exportNames.map((name): ExportOutcome => {
      try {
        return { ok: true, schema: convertExport(mod, name, filePath) };
      } catch (error) {
        return { ok: false, error };
      }
    });
  });
}

export async function evaluateSyntheticSchema(
  source: string,
  exportName: string,
  baseFilePath: string,
  sharedWorkspace?: EvalWorkspace
): Promise<Record<string, unknown>> {
  const workspace = sharedWorkspace ?? (await createEvalWorkspace());
  try {
    const filePath = await workspace.newModulePath(baseFilePath);
    await writeFile(filePath, source, 'utf8');
    return await evaluateSchema(filePath, exportName);
  } finally {
    if (!sharedWorkspace) await workspace.dispose();
  }
}

let redirectDepth = 0;
let originalStdoutWrite: typeof process.stdout.write | undefined;

/**
 * Redirect stdout to stderr during evaluation so user-code console.log()
 * calls don't corrupt the JSON-RPC protocol channel. Reentrant and safe for
 * overlapping calls: stdout is restored when the last one finishes.
 */
export async function withStdoutRedirect<T>(
  fn: () => Promise<T>
): Promise<T> {
  if (redirectDepth++ === 0) {
    originalStdoutWrite = process.stdout.write;
    process.stdout.write = process.stderr.write.bind(
      process.stderr
    ) as typeof process.stdout.write;
  }
  try {
    return await fn();
  } finally {
    if (--redirectDepth === 0) process.stdout.write = originalStdoutWrite!;
  }
}

async function importModule(filePath: string): Promise<Record<string, unknown>> {
  const importPath = filePath.startsWith('file://')
    ? filePath
    : pathToFileURL(filePath).href;
  return (await import(importPath)) as Record<string, unknown>;
}

async function linkNodeModules(dir: string, nodeModules: string): Promise<void> {
  try {
    await symlink(nodeModules, join(dir, 'node_modules'), 'dir');
  } catch (err) {
    // Bare imports may still be rewritten to file URLs. A symlink failure should
    // not prevent evaluating schemas that do not need package resolution.
    const msg = err instanceof Error ? err.message : String(err);
    console.warn(`[schemalint-zod] Could not symlink node_modules into temp dir: ${msg}`);
  }
}

function findNearestNodeModules(filePath: string): string | undefined {
  let dir = dirname(filePath);
  while (true) {
    const candidate = join(dir, 'node_modules');
    if (existsSync(candidate)) return candidate;

    const parent = dirname(dir);
    if (parent === dir) return undefined;
    dir = parent;
  }
}

/**
 * Convert a Zod schema to JSON Schema.
 *
 * Detects Zod v4's native `toJSONSchema()` method and uses it when available.
 * Falls back to `zod-to-json-schema` for Zod v3.
 */
function zodToJsonSchema(
  schema: unknown,
  sourceFile: string
): Record<string, unknown> {
  if (!schema || typeof schema !== 'object') {
    throw new Error('Discovered value is not a Zod schema');
  }

  // Zod 4 and Zod Mini share the stable `_zod` core marker. Conversion is a
  // package-level API, so resolve it from the user's project, not this helper.
  if ('_zod' in schema) {
    // `zod/v4/core` is Zod's permanent path to the v4 engine: from 3.25 (where
    // the package root is still v3) through every 4.x, for classic and mini.
    const userRequire = createRequire(pathToFileURL(sourceFile));
    const zod = ['zod/v4/core', 'zod']
      .map((id) => {
        try {
          return userRequire(id) as { toJSONSchema?: (value: unknown) => unknown };
        } catch {
          return {};
        }
      })
      .find((candidate) => typeof candidate.toJSONSchema === 'function') ?? {};
    if (typeof zod.toJSONSchema !== 'function') {
      throw new Error(
        "Zod v4 schema detected, but its package does not export toJSONSchema"
      );
    }
    return zod.toJSONSchema(schema) as Record<string, unknown>;
  }

  if (!('_def' in schema)) {
    throw new Error('Discovered value is not a recognized Zod v3/v4 schema');
  }

  // Zod v3 conversion remains helper-owned.
  // Use createRequire to load from the helper's own node_modules.
  const zodToJsonSchemaModule = localRequire('zod-to-json-schema') as {
    zodToJsonSchema: (s: unknown) => unknown;
  };
  return zodToJsonSchemaModule.zodToJsonSchema(schema) as Record<
    string,
    unknown
  >;
}
