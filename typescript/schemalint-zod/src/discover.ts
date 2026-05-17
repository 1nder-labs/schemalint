import { evaluateSchema, evaluateSyntheticSchema } from './evaluate.js';
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
  const isMatch = pm.default(sourceGlob, { dot: true }) as (
    input: string
  ) => boolean;
  const projectRoot = process.cwd();
  fileNames = fileNames.filter((f) => {
    const relPath = f.startsWith(projectRoot)
      ? f.slice(projectRoot.length + 1)
      : f;
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
  const discoveredLocations: SchemaTarget[] = [...callsiteTargets];

  const nonFatal: DiscoveryWarning[] = [];

  if (discoveredLocations.length === 0) {
    for (const sourceFile of selectedSourceFiles) {
      const exports = findExportedSchemaCalls(sourceFile, tsModule);
      for (const exp of exports) {
        const sourceMap = buildSourceMapFromObjectLiteral(
          exp.objectArg,
          sourceFile,
          tsModule
        );
        discoveredLocations.push({
          name: exp.name,
          filePath: sourceFile.fileName,
          exportName: exp.name,
          sourceMap,
        });
      }
    }
  }

  if (discoveredLocations.length === 0) {
    return { models: [], warnings: nonFatal };
  }

  // Step 3: Runtime evaluation — import each file and evaluate schemas
  const models: DiscoveredModel[] = [];

  for (const loc of discoveredLocations) {
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
      nonFatal.push({
        model: loc.name,
        message: `Failed to evaluate schema '${loc.name}' in ${loc.filePath}: ${message}`,
      });
    }
  }

  const response: DiscoverResponse = { models, warnings: nonFatal };
  if (providerHint) {
    response.provider_hint = providerHint;
  }
  return response;
}
