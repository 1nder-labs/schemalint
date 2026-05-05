import type * as ts from 'typescript';

import { evaluateSchema } from './evaluate.js';

interface SourceMapEntry {
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

interface DiscoveredSchemaLocation {
  name: string;
  filePath: string;
  sourceMap: Record<string, SourceMapEntry>;
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

  // Step 0: Scan provider imports for auto-detection
  let providerHint: string | undefined;
  for (const sourceFile of program.getSourceFiles()) {
    if (sourceFile.isDeclarationFile || sourceFile.fileName.includes('node_modules')) continue;
    if (!fileNames.includes(sourceFile.fileName)) continue;
    const hint = scanProviderImports(sourceFile, tsModule);
    if (hint) {
      providerHint = hint;
      break;
    }
  }

  // Step 1: Collect zodTextFormat / zodResponseFormat references
  // These point to schemas that should be discovered even if not top-level exported.
  const zodFormatRefs: string[] = [];
  for (const sourceFile of program.getSourceFiles()) {
    if (sourceFile.isDeclarationFile || sourceFile.fileName.includes('node_modules')) continue;
    if (!fileNames.includes(sourceFile.fileName)) continue;
    for (const ref of scanZodTextFormatRefs(sourceFile, tsModule)) {
      zodFormatRefs.push(ref);
    }
  }

  // Step 2: AST walk to find z.object() calls and extract source locations
  const discoveredLocations: DiscoveredSchemaLocation[] = [];
  const nonFatal: DiscoveryWarning[] = [];
  const zodFormatRefSet = new Set(zodFormatRefs);

  for (const sourceFile of program.getSourceFiles()) {
    // Skip lib files (e.g., node_modules, TypeScript DOM libs)
    if (
      sourceFile.isDeclarationFile ||
      sourceFile.fileName.includes('node_modules')
    ) {
      continue;
    }
    // Only walk files in our filtered list
    if (!fileNames.includes(sourceFile.fileName)) continue;

    const exports = findExportedSchemaCalls(sourceFile, tsModule, zodFormatRefSet);
    for (const exp of exports) {
      const sourceMap = buildSourceMapFromObjectLiteral(
        exp.objectArg,
        sourceFile,
        tsModule
      );
      discoveredLocations.push({
        name: exp.name,
        filePath: sourceFile.fileName,
        sourceMap,
      });
    }
  }

  if (discoveredLocations.length === 0) {
    return { models: [], warnings: nonFatal };
  }

  // Step 3: Runtime evaluation — import each file and evaluate schemas
  const models: DiscoveredModel[] = [];

  for (const loc of discoveredLocations) {
    try {
      const schemaJson = await evaluateSchema(loc.filePath, loc.name);
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

// ---------------------------------------------------------------------------
// AST walking helpers
// ---------------------------------------------------------------------------

interface ExportedSchemaCall {
  name: string;
  objectArg: ts.ObjectLiteralExpression;
}

/**
 * Find top-level `const` declarations that are `z.object({...})` calls.
 * If `zodFormatRefs` is provided, non-exported schemas referenced by
 * zodTextFormat/zodResponseFormat are also included.
 */
function findExportedSchemaCalls(
  sourceFile: ts.SourceFile,
  tsModule: typeof ts,
  zodFormatRefs?: Set<string>
): ExportedSchemaCall[] {
  const results: ExportedSchemaCall[] = [];
  // Collect all const declarations (both exported and non-exported) for zodFormatRef matching
  const allConstDeclarations: Map<string, ExportedSchemaCall> = new Map();

  function walk(node: ts.Node): void {
    if (
      tsModule.isVariableStatement(node) &&
      node.declarationList.declarations.length === 1
    ) {
      const decl = node.declarationList.declarations[0];
      if (
        tsModule.isIdentifier(decl.name) &&
        decl.initializer
      ) {
        const call = findZObjectCall(decl.initializer, tsModule);
        if (call) {
          const entry: ExportedSchemaCall = {
            name: decl.name.text,
            objectArg: call,
          };
          allConstDeclarations.set(decl.name.text, entry);
          if (hasExportModifier(node, tsModule)) {
            results.push(entry);
          }
        }
      }
    }

    // Also handle `export default z.object({...})`
    if (
      tsModule.isExportAssignment(node) &&
      !node.isExportEquals &&
      tsModule.isCallExpression(node.expression)
    ) {
      const call = findZObjectCall(node.expression, tsModule);
      if (call) {
        const entry = {
          name: 'default',
          objectArg: call,
        };
        results.push(entry);
        allConstDeclarations.set('default', entry);
      }
    }

    tsModule.forEachChild(node, walk);
  }

  tsModule.forEachChild(sourceFile, walk);

  // Add schemas referenced in zodTextFormat/zodResponseFormat calls that
  // are not exported but are declared in this file (non-exported schemas
  // referenced by format helpers).
  if (zodFormatRefs && zodFormatRefs.size > 0) {
    for (const name of zodFormatRefs) {
      const decl = allConstDeclarations.get(name);
      if (decl && !results.some(r => r.name === name)) {
        results.push(decl);
      }
    }
  }

  return results;
}

/**
 * Scan for zodTextFormat(MySchema, "name") and zodResponseFormat(MySchema, "name")
 * call expressions. Returns the list of schema names referenced as the first argument.
 */
function scanZodTextFormatRefs(
  sourceFile: ts.SourceFile,
  tsModule: typeof ts
): string[] {
  const refs: string[] = [];

  function walk(node: ts.Node): void {
    if (!tsModule.isCallExpression(node)) {
      tsModule.forEachChild(node, walk);
      return;
    }
    // Match zodTextFormat(...) or zodResponseFormat(...)
    if (tsModule.isIdentifier(node.expression)) {
      const name = node.expression.text;
      if (
        (name === 'zodTextFormat' || name === 'zodResponseFormat') &&
        node.arguments.length >= 1 &&
        tsModule.isIdentifier(node.arguments[0])
      ) {
        refs.push(node.arguments[0].text);
        return; // no need to walk children of this call
      }
    }
    tsModule.forEachChild(node, walk);
  }

  tsModule.forEachChild(sourceFile, walk);
  return refs;
}

/**
 * Scan a source file's import declarations for provider SDKs.
 * Returns "openai" or "anthropic" if detected, undefined otherwise.
 */
function scanProviderImports(
  sourceFile: ts.SourceFile,
  tsModule: typeof ts
): string | undefined {
  for (const stmt of sourceFile.statements) {
    if (!tsModule.isImportDeclaration(stmt)) continue;
    const spec = stmt.moduleSpecifier;
    if (!tsModule.isStringLiteral(spec)) continue;
    const mod = spec.text;
    if (mod === 'openai' || mod.startsWith('openai/')) {
      return 'openai';
    }
    if (mod === '@anthropic-ai/sdk' || mod.startsWith('@anthropic-ai/')) {
      return 'anthropic';
    }
  }
  return undefined;
}

function hasExportModifier(
  node: ts.Node,
  tsModule: typeof ts
): boolean {
  if (!tsModule.canHaveModifiers(node)) return false;
  const modifiers = tsModule.getModifiers(node);
  if (!modifiers) return false;
  for (const mod of modifiers) {
    if (mod.kind === tsModule.SyntaxKind.ExportKeyword) return true;
  }
  return false;
}

/**
 * Given a node, if it is `z.object({...})`, return the ObjectLiteralExpression argument.
 * Handles chaining: `z.object({...}).extend({...})` — returns the initial object.
 */
function findZObjectCall(
  node: ts.Node,
  tsModule: typeof ts
): ts.ObjectLiteralExpression | null {
  // Unwrap parenthesized expressions
  while (tsModule.isParenthesizedExpression(node)) {
    node = node.expression;
  }

  // Handle `export default z.object({...})` wrapped in another call expression
  // e.g., z.object({...}).extend({...})
  if (
    tsModule.isCallExpression(node) &&
    tsModule.isPropertyAccessExpression(node.expression)
  ) {
    // Check if the inner node is a z.object() call
    const innerNode = node.expression.expression;
    if (
      tsModule.isCallExpression(innerNode) &&
      isZObjectCallExpression(innerNode, tsModule)
    ) {
      const prop = innerNode.arguments[0];
      if (prop && tsModule.isObjectLiteralExpression(prop)) {
        return prop;
      }
    }
    // Check for .extend() / .merge() / .pick() / .omit() chaining on z.object()
    if (tsModule.isIdentifier(node.expression.name)) {
      const methodName = node.expression.name.text;
      if (
        methodName === 'extend' ||
        methodName === 'merge' ||
        methodName === 'pick' ||
        methodName === 'omit'
      ) {
        return findZObjectCall(node.expression.expression, tsModule);
      }
    }
  }

  // Direct z.object() call
  if (
    tsModule.isCallExpression(node) &&
    isZObjectCallExpression(node, tsModule)
  ) {
    const arg = node.arguments[0];
    if (arg && tsModule.isObjectLiteralExpression(arg)) {
      return arg;
    }
  }

  return null;
}

/**
 * Check if a CallExpression is `z.object(...)`.
 */
function isZObjectCallExpression(
  node: ts.CallExpression,
  tsModule: typeof ts
): boolean {
  const expr = node.expression;
  if (!tsModule.isPropertyAccessExpression(expr)) return false;
  return (
    tsModule.isIdentifier(expr.expression) &&
    expr.expression.text === 'z' &&
    tsModule.isIdentifier(expr.name) &&
    expr.name.text === 'object'
  );
}

/**
 * Walk an ObjectLiteralExpression (`{ email: z.string(), ... }`) and build a
 * source map mapping JSON Pointer paths to file:line locations.
 *
 * Handles nested `z.object({...})` by recursing into inner object literals.
 */
function buildSourceMapFromObjectLiteral(
  objLit: ts.ObjectLiteralExpression,
  sourceFile: ts.SourceFile,
  tsModule: typeof ts
): Record<string, SourceMapEntry> {
  const map: Record<string, SourceMapEntry> = {};

  for (const prop of objLit.properties) {
    if (
      !tsModule.isPropertyAssignment(prop) ||
      !tsModule.isIdentifier(prop.name)
    ) {
      continue;
    }
    const propName = prop.name.text;
    const { line } = sourceFile.getLineAndCharacterOfPosition(
      prop.getStart(sourceFile)
    );
    const pointer = `/properties/${propName}`;
    map[pointer] = {
      file: sourceFile.fileName,
      line: line + 1, // 1-indexed
    };

    // Recurse into nested z.object() values
    const innerCall = findZObjectCall(prop.initializer, tsModule);
    if (innerCall) {
      const nested = buildSourceMapFromObjectLiteral(
        innerCall,
        sourceFile,
        tsModule
      );
      for (const [nestedPointer, nestedSpan] of Object.entries(nested)) {
        map[`/properties/${propName}${nestedPointer}`] = nestedSpan;
      }
    }
  }

  return map;
}
