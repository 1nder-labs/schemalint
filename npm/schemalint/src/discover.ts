import { realpathSync } from 'node:fs';
import path from 'node:path';

import type * as ts from 'typescript';

import {
  createEvalWorkspace,
  evaluateSyntheticExports,
  evaluateSyntheticSchema,
  withStdoutRedirect,
  type EvalWorkspace,
} from './evaluate.js';
import { groupByProject, listScopeFiles } from './projects.js';
import { findSchemaRoots, type SchemaRoot } from './schema_roots.js';
import { buildSlicedModule, buildSlicedModuleBatch, safeName } from './slice.js';
import { resolveSourceScope } from './source_scope.js';

/**
 * Normalize a file-system path to forward slashes.
 *
 * `path.relative()` returns backslash-separated paths on Windows
 * (e.g. `src\\foo.ts`), but picomatch globs use forward slashes.
 * Replacing the OS separator with `/` is a no-op on POSIX and correct
 * on Windows, making the glob filter work on both platforms.
 *
 * The `sep` parameter exists solely for unit-testing Windows paths on a
 * POSIX machine — pass `'\\'` to simulate Windows `path.sep`.
 */
export function toPosixPath(p: string, sep: string = path.sep): string {
  return sep === '/' ? p : p.split(sep).join('/');
}

export interface SourceMapEntry {
  file: string;
  line?: number;
}

export interface TargetSpan {
  file: string;
  line: number;
  col: number;
}

/**
 * The wire shape the Rust CLI reads. `provider`, `envelope`, and
 * `canonical_kind` are constant: discovery no longer knows which SDK a
 * schema reaches, so the profile comes from `--profile` or package.json.
 */
export interface DiscoveredModel {
  name: string;
  module_path: string;
  schema: Record<string, unknown>;
  source_map: Record<string, SourceMapEntry>;
  canonical_kind: 'zod.export';
  provider: { certainty: 'ambiguous' };
  envelope: Record<string, never>;
  usage_span: TargetSpan;
}

export interface DiscoveryWarning {
  model: string;
  message: string;
}

export interface DiscoveryFailure {
  kind: 'evaluation';
  target: string;
  message: string;
}

export interface DiscoveryCounts {
  attempted: number;
  excluded: number;
  discovered: number;
  failed: number;
}

export interface DiscoverResponse {
  models: DiscoveredModel[];
  warnings: DiscoveryWarning[];
  failures: DiscoveryFailure[];
  counts: DiscoveryCounts;
}

// ponytail: Zod 4's own wording for a type with no JSON Schema form (Date,
// transforms, custom types). Zod 3's converter never throws it.
const UNREPRESENTABLE = 'cannot be represented in JSON Schema';

interface Evaluated {
  models: DiscoveredModel[];
  failures: DiscoveryFailure[];
  skipped: DiscoveryWarning[];
}

type RootOutcome =
  | { schema: Record<string, unknown> }
  | { error: unknown };

/**
 * Slice every root of one declaring file into a single module and evaluate it
 * with one import. If the batch cannot be built or imported (one seed's
 * closure throwing at module level poisons the rest), evaluate each root on its
 * own so every result matches the per-root outcome.
 */
async function evaluateFile(
  fileRoots: readonly SchemaRoot[],
  compilerOptions: ts.CompilerOptions,
  tsModule: typeof ts,
  workspace: EvalWorkspace
): Promise<RootOutcome[]> {
  const filePath = fileRoots[0].sourceFile.fileName;
  const exportNameOf = (root: SchemaRoot, index?: number): string =>
    `__schemalint_target_${safeName(root.name)}${index === undefined ? '' : `_${index}`}`;
  try {
    const exportNames = fileRoots.map((root, i) => exportNameOf(root, i));
    const source = buildSlicedModuleBatch(
      fileRoots[0].sourceFile,
      fileRoots.map((root, i) => ({ seed: root.seed, exportName: exportNames[i] })),
      tsModule,
      compilerOptions
    );
    const results = await evaluateSyntheticExports(source, exportNames, filePath, workspace);
    return results.map((r) => (r.ok ? { schema: r.schema } : { error: r.error }));
  } catch {
    const outcomes: RootOutcome[] = [];
    for (const root of fileRoots) {
      const exportName = exportNameOf(root);
      try {
        const source = buildSlicedModule(
          root.sourceFile,
          root.seed,
          exportName,
          tsModule,
          compilerOptions
        );
        outcomes.push({
          schema: await evaluateSyntheticSchema(source, exportName, filePath, workspace),
        });
      } catch (error) {
        outcomes.push({ error });
      }
    }
    return outcomes;
  }
}

/** Slice each schema out of its declaring file and convert it to JSON Schema. */
async function evaluateRoots(
  roots: readonly SchemaRoot[],
  compilerOptions: ts.CompilerOptions,
  tsModule: typeof ts
): Promise<Evaluated> {
  const out: Evaluated = { models: [], failures: [], skipped: [] };
  const byFile = new Map<ts.SourceFile, number[]>();
  roots.forEach((root, i) => {
    const indexes = byFile.get(root.sourceFile);
    if (indexes) indexes.push(i);
    else byFile.set(root.sourceFile, [i]);
  });

  const outcomes: RootOutcome[] = new Array(roots.length);
  const workspace = await createEvalWorkspace();
  try {
    await withStdoutRedirect(async () => {
      // Sequential: measured, importing files 8 at a time was not faster.
      for (const indexes of byFile.values()) {
        const results = await evaluateFile(
          indexes.map((i) => roots[i]),
          compilerOptions,
          tsModule,
          workspace
        );
        indexes.forEach((rootIndex, k) => (outcomes[rootIndex] = results[k]));
      }
    });
  } finally {
    await workspace.dispose();
  }

  roots.forEach((root, i) => {
    const filePath = root.sourceFile.fileName;
    const outcome = outcomes[i];
    if ('schema' in outcome) {
      const line = root.sourceMap['']?.line ?? 1;
      out.models.push({
        name: root.name,
        module_path: filePath,
        schema: outcome.schema,
        source_map: root.sourceMap,
        canonical_kind: 'zod.export',
        provider: { certainty: 'ambiguous' },
        envelope: {},
        usage_span: { file: filePath, line, col: 1 },
      });
      return;
    }
    const err = outcome.error;
    const message = `Failed to evaluate schema '${root.name}' in ${filePath}: ${
      err instanceof Error ? err.message : String(err)
    }`;
    // A schema Zod cannot express as JSON Schema cannot be sent to a model
    // as structured output, so it is reported and left out of coverage.
    if (message.includes(UNREPRESENTABLE)) {
      out.skipped.push({ model: root.name, message: `Skipped: ${message}` });
    } else {
      out.failures.push({ kind: 'evaluation', target: root.name, message });
    }
  });
  return out;
}

/**
 * Discover Zod schemas by walking TypeScript ASTs.
 *
 * 1. Lists the TypeScript files the source scope names, from disk.
 * 2. Groups them by nearest tsconfig.json and builds one program per group;
 *    files with no tsconfig above them share a default program.
 * 3. Finds every Zod object schema those files build (see `findSchemaRoots`).
 * 4. Slices each out of its declaring file and converts it to JSON Schema.
 */
export async function discoverZodSchemas(
  sourceGlob: string,
  exclusions: string[] = []
): Promise<DiscoverResponse> {
  const tsModule = await import('typescript');
  const pm = await import('picomatch');

  const picomatch =
    typeof pm.default === 'function' ? pm.default : (pm as unknown as typeof pm.default);
  if (typeof picomatch !== 'function') {
    throw new Error(
      'Failed to load picomatch: expected a function but got ' +
        typeof picomatch +
        '. Check that picomatch is correctly installed.'
    );
  }
  // Real path on both sides: files are listed and compared as real paths, so
  // a symlinked working directory (macOS /tmp) still matches its own files.
  const projectRoot = realpathSync(process.cwd());
  const scope = resolveSourceScope(sourceGlob, picomatch, projectRoot);
  const matched = listScopeFiles(scope, projectRoot, tsModule, picomatch);
  const excludeMatchers = exclusions.map(
    (pattern) => picomatch(pattern, { dot: true }) as (input: string) => boolean
  );
  const fileNames = matched.filter((file) => {
    const relative = toPosixPath(path.relative(projectRoot, file));
    return !excludeMatchers.some((matches) => matches(relative));
  });
  const excluded = matched.length - fileNames.length;
  const empty = (warnings: DiscoveryWarning[]): DiscoverResponse => ({
    models: [],
    warnings,
    failures: [],
    counts: { attempted: 0, excluded, discovered: 0, failed: 0 },
  });

  if (fileNames.length === 0) {
    // Files removed by --exclude are ordinary exclusion, not a discovery problem.
    return empty(
      matched.length === 0
        ? [{ model: '', message: `No file on disk matched source glob '${scope.pattern}'.` }]
        : []
    );
  }

  // A named file bypasses tsconfig include/exclude; a directory or glob does not.
  const groups = groupByProject(fileNames, tsModule, scope.target?.kind !== 'file');
  if (groups.length === 0) {
    return empty([
      {
        model: '',
        message:
          `${fileNames.length} file(s) matched source glob '${sourceGlob}' ` +
          'but all are excluded by tsconfig.json. Name a file directly to check it.',
      },
    ]);
  }

  const seen = new Set<string>();
  const result: Evaluated = { models: [], failures: [], skipped: [] };
  let attempted = 0;
  for (const group of groups) {
    const program = tsModule.createProgram(group.files, group.compilerOptions);
    const selected = new Set(group.files);
    const sourceFiles = program
      .getSourceFiles()
      .filter((file) => !file.isDeclarationFile && selected.has(file.fileName));
    const roots = findSchemaRoots(sourceFiles, program.getTypeChecker(), tsModule).filter(
      (root) => !seen.has(root.key) && seen.add(root.key)
    );
    const evaluated = await evaluateRoots(roots, group.compilerOptions, tsModule);
    attempted += roots.length - evaluated.skipped.length;
    result.models.push(...evaluated.models);
    result.failures.push(...evaluated.failures);
    result.skipped.push(...evaluated.skipped);
  }

  if (attempted === 0 && result.skipped.length === 0) {
    return empty([
      {
        model: '',
        message:
          `Checked ${fileNames.length} file(s) matched by source glob ` +
          `'${sourceGlob}' for Zod schemas but found none.`,
      },
    ]);
  }
  return {
    models: result.models,
    warnings: result.skipped,
    failures: result.failures,
    counts: {
      attempted,
      excluded,
      discovered: result.models.length,
      failed: result.failures.length,
    },
  };
}
