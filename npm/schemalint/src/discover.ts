import path from 'node:path';

import type * as ts from 'typescript';

import { evaluateSchema, evaluateSyntheticSchema } from './evaluate.js';

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
import {
  buildSourceMapFromObjectLiteral,
  findExportedSchemaCalls,
  scanProviderImports,
} from './discover_ast.js';
import { findSchemaTargets, type SchemaTarget } from './targets.js';

export interface SourceMapEntry {
  file: string;
  line?: number;
}

export interface DiscoveredModel {
  name: string;
  module_path: string;
  schema: Record<string, unknown>;
  source_map: Record<string, SourceMapEntry>;
}

export interface DiscoveryWarning {
  model: string;
  message: string;
}

export interface DiscoverResponse {
  models: DiscoveredModel[];
  warnings: DiscoveryWarning[];
  provider_hint?: string;
}

/** Import each target and convert it to JSON Schema, recording per-target failures. */
async function evaluateTargets(
  locations: readonly SchemaTarget[],
  warnings: DiscoveryWarning[]
): Promise<DiscoveredModel[]> {
  const models: DiscoveredModel[] = [];
  for (const loc of locations) {
    try {
      const schemaJson = loc.syntheticSource
        ? await evaluateSyntheticSchema(
            loc.syntheticSource,
            loc.exportName,
            loc.filePath
          )
        : await evaluateSchema(loc.filePath, loc.exportName);
      models.push({
        name: loc.name,
        module_path: loc.filePath,
        schema: schemaJson as Record<string, unknown>,
        source_map: loc.sourceMap,
      });
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      warnings.push({
        model: loc.name,
        message: `Failed to evaluate schema '${loc.name}' in ${loc.filePath}: ${message}`,
      });
    }
  }
  return models;
}

/** Every top-level exported `z.object({...})`, the fallback discovery source. */
function exportedSchemaTargets(
  sourceFiles: readonly ts.SourceFile[],
  tsModule: typeof ts
): SchemaTarget[] {
  const targets: SchemaTarget[] = [];
  for (const sourceFile of sourceFiles) {
    for (const exp of findExportedSchemaCalls(sourceFile, tsModule)) {
      targets.push({
        name: exp.name,
        filePath: sourceFile.fileName,
        exportName: exp.name,
        sourceMap: buildSourceMapFromObjectLiteral(
          exp.objectArg,
          sourceFile,
          tsModule
        ),
      });
    }
  }
  return targets;
}

/**
 * Discover Zod schemas by walking TypeScript ASTs.
 *
 * 1. Reads tsconfig.json to resolve the project file list.
 * 2. Filters files against the user-supplied source glob.
 * 3. Walks each source file's AST looking for `z.object({...})` calls.
 * 4. Extracts property source locations for source map.
 * 5. Dynamically imports each file and evaluates schemas at runtime.
 * 6. Converts schemas to JSON Schema via zod-to-json-schema or native.
 */
export async function discoverZodSchemas(
  sourceGlob: string
): Promise<DiscoverResponse> {
  const tsModule = await import('typescript');
  const pm = await import('picomatch');

  // Resolve tsconfig.json
  const configPath = tsModule.findConfigFile(
    process.cwd(),
    tsModule.sys.fileExists,
    'tsconfig.json'
  );
  if (!configPath) {
    throw new Error(
      'No tsconfig.json found in the current project. ' +
        'Run this tool from a TypeScript project root.'
    );
  }

  const configFile = tsModule.readConfigFile(configPath, tsModule.sys.readFile);
  if (configFile.error) {
    throw new Error(
      `Failed to read tsconfig.json: ${tsModule.flattenDiagnosticMessageText(
        configFile.error.messageText,
        '\n'
      )}`
    );
  }

  const parsedConfig = tsModule.parseJsonConfigFileContent(
    configFile.config,
    tsModule.sys,
    process.cwd()
  );
  const compilerOptions = parsedConfig.options;

  // Resolve the full file list from tsconfig
  let fileNames: string[] = parsedConfig.fileNames;
  if (fileNames.length === 0) {
    throw new Error(
      'No source files found in tsconfig.json. Ensure "include" patterns match your project files.'
    );
  }

  // Filter files against the source glob.
  // tsconfig resolves absolute paths, but glob patterns match relative paths.
  // Convert each file to a path relative to the project root before matching.
  const picomatch =
    typeof pm.default === 'function' ? pm.default : (pm as unknown as typeof pm.default);
  if (typeof picomatch !== 'function') {
    throw new Error(
      'Failed to load picomatch: expected a function but got ' +
        typeof picomatch +
        '. Check that picomatch is correctly installed.'
    );
  }
  const isMatch = picomatch(sourceGlob, { dot: true }) as (
    input: string
  ) => boolean;
  const projectRoot = process.cwd();
  fileNames = fileNames.filter((f) => {
    // Use path.relative so that:
    //  1. Files exactly under projectRoot get a clean relative path ("src/foo.ts")
    //     without the boundary bug where startsWith("/repo/app") also matches
    //     "/repo/application/foo.ts".
    //  2. Files outside projectRoot (monorepo tsconfig referencing "../shared/…")
    //     get a "../"-prefixed path that the caller's glob can match if desired.
    //
    // Normalize to forward slashes before matching: on Windows, path.relative()
    // returns backslash-separated paths (e.g. "src\\foo.ts") but picomatch
    // globs use forward slashes, causing every file to fail to match.
    // toPosixPath() is a no-op on POSIX (path.sep === '/') and correct on Windows.
    const relPath = toPosixPath(path.relative(projectRoot, f));
    return isMatch(relPath);
  });

  if (fileNames.length === 0) {
    return { models: [], warnings: [] };
  }

  // Create program and walk ASTs to discover schemas
  const program = tsModule.createProgram(fileNames, compilerOptions);
  const fileSet = new Set(fileNames);
  const selectedSourceFiles = program.getSourceFiles().filter(
    (sourceFile) =>
      !sourceFile.isDeclarationFile &&
      !sourceFile.fileName.includes('node_modules') &&
      fileSet.has(sourceFile.fileName)
  );

  // Step 0: Scan provider imports for auto-detection
  let providerHint: string | undefined;
  for (const sourceFile of selectedSourceFiles) {
    const hint = scanProviderImports(sourceFile, tsModule);
    if (hint) {
      providerHint = hint;
      break;
    }
  }

  // Step 1: Prefer provider-facing call sites. This catches schemas passed to
  // AI SDK, OpenAI helpers, and Anthropic helper APIs. Legacy exported-schema
  // discovery remains as a fallback for simple projects and explicit schema
  // modules that are not wired to a provider call in the selected source glob.
  const callsiteTargets = findSchemaTargets(
    program,
    fileSet,
    tsModule,
    compilerOptions
  );
  const nonFatal: DiscoveryWarning[] = [];
  const models = await evaluateTargets(callsiteTargets, nonFatal);

  // Legacy exported-schema discovery, for projects with no provider call site
  // in the source glob. It keys off evaluated models rather than resolved
  // targets: a target that resolves but then fails to evaluate would otherwise
  // hold this gate shut and leave the run linting nothing at all.
  if (models.length === 0) {
    const exported = exportedSchemaTargets(selectedSourceFiles, tsModule);
    models.push(...(await evaluateTargets(exported, nonFatal)));
  }

  const response: DiscoverResponse = { models, warnings: nonFatal };
  if (providerHint) {
    response.provider_hint = providerHint;
  }
  return response;
}
