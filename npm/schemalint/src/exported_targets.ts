import type * as ts from 'typescript';

import {
  buildRootSourceMap,
  buildSourceMapFromObjectLiteral,
  findExportedSchemaCalls,
  findZObjectCall,
  OBJECT_CHAIN_METHODS,
} from './discover_ast.js';
import { buildSlicedModule, safeName } from './slice.js';
import { staticAlternatives } from './static_expression.js';
import { declarationKey } from './target_emit.js';
import type { SchemaTarget } from './targets.js';

interface ExportedSchema {
  name: string;
  /** File declaring the exported name, which may differ from the scanned one. */
  sourceFile: ts.SourceFile;
  seedExpr: ts.Expression;
  objectArg: ts.ObjectLiteralExpression;
  objectFile: ts.SourceFile;
}

/**
 * Every exported `z.object({...})` schema reachable from `sourceFiles`, the
 * fallback discovery source. Re-exports (`export * from`, `export { X } from`)
 * are followed to the declaring file, and a schema reached more than once is
 * reported once.
 */
export function exportedSchemaTargets(
  sourceFiles: readonly ts.SourceFile[],
  checker: ts.TypeChecker,
  tsModule: typeof ts,
  compilerOptions: ts.CompilerOptions
): SchemaTarget[] {
  const targets: SchemaTarget[] = [];
  const seen = new Set<string>();
  for (const sourceFile of sourceFiles) {
    for (const exp of [
      ...directExports(sourceFile, tsModule),
      ...moduleExports(sourceFile, checker, tsModule),
    ]) {
      const declaration = declarationKey(exp.sourceFile.fileName, exp.name);
      if (seen.has(declaration)) continue;
      seen.add(declaration);
      targets.push(toTarget(exp, declaration, tsModule, compilerOptions));
    }
  }
  return targets;
}

function directExports(
  sourceFile: ts.SourceFile,
  tsModule: typeof ts
): ExportedSchema[] {
  return findExportedSchemaCalls(sourceFile, tsModule).map((exp) => ({
    name: exp.name,
    sourceFile,
    seedExpr: exp.seedExpr,
    objectArg: exp.objectArg,
    objectFile: sourceFile,
  }));
}

/**
 * Exports the checker sees in `sourceFile`, including re-exports and names
 * bound to another schema (`const Inner = z.object(...); export const B = Inner`).
 */
function moduleExports(
  sourceFile: ts.SourceFile,
  checker: ts.TypeChecker,
  tsModule: typeof ts
): ExportedSchema[] {
  const moduleSymbol = checker.getSymbolAtLocation(sourceFile);
  if (!moduleSymbol) return [];
  const found: ExportedSchema[] = [];
  for (const exported of checker.getExportsOfModule(moduleSymbol)) {
    const symbol =
      exported.flags & tsModule.SymbolFlags.Alias
        ? checker.getAliasedSymbol(exported)
        : exported;
    const decl = symbol.valueDeclaration;
    if (
      !decl ||
      !tsModule.isVariableDeclaration(decl) ||
      !tsModule.isIdentifier(decl.name) ||
      !decl.initializer
    ) {
      continue;
    }
    const declFile = decl.getSourceFile();
    if (declFile.isDeclarationFile || declFile.fileName.includes('node_modules')) {
      continue;
    }
    const schema = zObjectThroughAliases(decl.initializer, checker, tsModule, 0);
    if (!schema) continue;
    found.push({
      name: decl.name.text,
      sourceFile: declFile,
      seedExpr: decl.name,
      objectArg: schema.call,
      objectFile: schema.sourceFile,
    });
  }
  return found;
}

/**
 * The `z.object({...})` literal an expression evaluates to, following
 * identifier aliases (across imports) with the same resolution the wrapper
 * tracing uses, and through `.extend()`-style chains on an aliased schema.
 */
function zObjectThroughAliases(
  expr: ts.Expression,
  checker: ts.TypeChecker,
  tsModule: typeof ts,
  depth: number
): { call: ts.ObjectLiteralExpression; sourceFile: ts.SourceFile } | undefined {
  const alternatives = staticAlternatives(expr, checker, tsModule);
  if (alternatives.length !== 1 || depth > 8) return undefined;
  const resolved = alternatives[0];
  const call = findZObjectCall(resolved, tsModule);
  if (call) return { call, sourceFile: resolved.getSourceFile() };
  if (
    tsModule.isCallExpression(resolved) &&
    tsModule.isPropertyAccessExpression(resolved.expression) &&
    OBJECT_CHAIN_METHODS.has(resolved.expression.name.text)
  ) {
    return zObjectThroughAliases(
      resolved.expression.expression,
      checker,
      tsModule,
      depth + 1
    );
  }
  return undefined;
}

function toTarget(
  exp: ExportedSchema,
  declaration: string,
  tsModule: typeof ts,
  compilerOptions: ts.CompilerOptions
): SchemaTarget {
  const sourceMap = {
    ...buildRootSourceMap(exp.seedExpr, exp.sourceFile),
    ...buildSourceMapFromObjectLiteral(exp.objectArg, exp.objectFile, tsModule),
  };
  const exportName = `__schemalint_target_${safeName(exp.name)}`;
  return {
    name: exp.name,
    filePath: exp.sourceFile.fileName,
    exportName,
    sourceMap,
    canonicalKind: 'zod.export',
    provider: { certainty: 'ambiguous' },
    envelope: {},
    declaration,
    usageSpan: {
      file: exp.sourceFile.fileName,
      line: sourceMap['']?.line ?? 1,
      col: 1,
    },
    syntheticSource: buildSlicedModule(
      exp.sourceFile,
      exp.seedExpr,
      exportName,
      tsModule,
      compilerOptions
    ),
  };
}
