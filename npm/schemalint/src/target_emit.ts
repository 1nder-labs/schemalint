import { pathToFileURL } from 'node:url';

import type * as ts from 'typescript';

import type { SourceMapEntry } from './discover.js';
import {
  buildRootSourceMap,
  buildSourceMapFromObjectLiteral,
  findZObjectCall,
  hasExportModifier,
} from './discover_ast.js';
import type { TargetExpression } from './target_resolution.js';
import {
  resolveVariableDeclaration,
  unwrapExpression,
} from './static_expression.js';
import type {
  EnvelopeField,
  ProviderResolution,
  TargetSpan,
} from './sdk_adapters.js';

export interface SchemaTarget {
  name: string;
  filePath: string;
  exportName: string;
  sourceMap: Record<string, SourceMapEntry>;
  canonicalKind: string;
  provider: ProviderResolution;
  envelope: Record<string, EnvelopeField>;
  usageSpan: TargetSpan;
  syntheticSource?: string;
}

export function resolveTarget(
  target: TargetExpression,
  checker: ts.TypeChecker,
  tsModule: typeof ts,
  compilerOptions: ts.CompilerOptions
): SchemaTarget {
  const expr = unwrapLocalAlias(target.expression, checker, tsModule);
  const sourceFile = target.sourceFile;
  const sourceMap = sourceMapForTarget(expr, sourceFile, checker, tsModule);

  if (tsModule.isIdentifier(expr)) {
    const exported = resolveExportedIdentifier(expr, checker, tsModule);
    if (exported) {
      return {
        name: target.name,
        filePath: exported.filePath,
        exportName: exported.exportName,
        sourceMap,
        canonicalKind: target.metadata.canonicalKind,
        provider: target.metadata.provider,
        envelope: target.metadata.envelope,
        usageSpan: target.metadata.usageSpan,
      };
    }
  }

  const exportName = `__schemalint_target_${safeName(target.name)}`;
  return {
    name: target.name,
    filePath: sourceFile.fileName,
    exportName,
    sourceMap,
    canonicalKind: target.metadata.canonicalKind,
    provider: target.metadata.provider,
    envelope: target.metadata.envelope,
    usageSpan: target.metadata.usageSpan,
    syntheticSource: buildSyntheticModule(
      sourceFile,
      expr,
      exportName,
      tsModule,
      compilerOptions,
      target.metadata.adapterModule
    ),
  };
}

/**
 * Follow a function-local `const schema = ...` to the expression it aliases.
 *
 * Only module-level declarations survive into the synthetic module (see
 * `isReusableDeclaration`), so a name bound inside a function body would be
 * emitted as an undefined reference. Its initializer is the real target, and
 * that initializer is either an inline expression or a module-level name the
 * synthetic module does hoist.
 */
function unwrapLocalAlias(
  expression: ts.Expression,
  checker: ts.TypeChecker,
  tsModule: typeof ts
): ts.Expression {
  let current = unwrapExpression(expression, tsModule);
  // ponytail: bounded rather than cycle-tracked; `const a = b, b = a` is not
  // valid code, so the only chains here are short alias hops.
  for (let hop = 0; hop < 8 && tsModule.isIdentifier(current); hop++) {
    const decl = resolveVariableDeclaration(current, checker, tsModule);
    if (!decl?.initializer) break;
    const stmt = decl.parent.parent;
    const moduleLevel =
      tsModule.isVariableStatement(stmt) && tsModule.isSourceFile(stmt.parent);
    if (moduleLevel) break;
    current = unwrapExpression(decl.initializer, tsModule);
  }
  return current;
}

function resolveExportedIdentifier(
  id: ts.Identifier,
  checker: ts.TypeChecker,
  tsModule: typeof ts
): { filePath: string; exportName: string } | undefined {
  const decl = resolveVariableDeclaration(id, checker, tsModule);
  if (!decl) return undefined;

  if (tsModule.isIdentifier(decl.name)) {
    const stmt = decl.parent.parent;
    if (tsModule.isVariableStatement(stmt) && hasExportModifier(stmt, tsModule)) {
      return {
        filePath: decl.getSourceFile().fileName,
        exportName: decl.name.text,
      };
    }
  }

  return undefined;
}

function buildSyntheticModule(
  sourceFile: ts.SourceFile,
  expr: ts.Expression,
  exportName: string,
  tsModule: typeof ts,
  compilerOptions: ts.CompilerOptions,
  adapterModule: string
): string {
  const parts: string[] = [];
  const adapterNames = importedNames(sourceFile, adapterModule, tsModule);

  for (const stmt of sourceFile.statements) {
    if (tsModule.isImportDeclaration(stmt)) {
      if (
        tsModule.isStringLiteral(stmt.moduleSpecifier) &&
        stmt.moduleSpecifier.text === adapterModule
      ) {
        continue;
      }
      parts.push(rewriteImport(stmt, sourceFile, tsModule, compilerOptions));
      continue;
    }
    // Keep every reusable declaration except those that use a name imported
    // from the adapter module, since that import is stripped above and the
    // statement could not run without it (and would call the provider SDK at
    // import time if it could).
    //
    // This used to drop the statement *containing* the target instead, which
    // is wrong whenever the target resolved into a declaration: a schema
    // reached through `const First = z.object(...)` lost that binding while
    // other retained declarations still referenced `First`, so the synthetic
    // module died with a ReferenceError. Whether that happened depended on
    // whether symbol resolution succeeded, which made it look intermittent.
    if (isReusableDeclaration(stmt, tsModule) && !usesAny(stmt, adapterNames, tsModule)) {
      parts.push(stmt.getText(sourceFile));
    }
  }
  parts.push(`export const ${exportName} = ${expr.getText(sourceFile)};`);
  return parts.join('\n\n');
}

/** Names this module imports from `moduleSpecifier`. */
function importedNames(
  sourceFile: ts.SourceFile,
  moduleSpecifier: string,
  tsModule: typeof ts
): Set<string> {
  const names = new Set<string>();
  for (const stmt of sourceFile.statements) {
    if (!tsModule.isImportDeclaration(stmt)) continue;
    if (!tsModule.isStringLiteral(stmt.moduleSpecifier)) continue;
    if (stmt.moduleSpecifier.text !== moduleSpecifier) continue;
    const clause = stmt.importClause;
    if (clause?.name) names.add(clause.name.text);
    const bindings = clause?.namedBindings;
    if (bindings && tsModule.isNamedImports(bindings)) {
      for (const element of bindings.elements) names.add(element.name.text);
    }
    if (bindings && tsModule.isNamespaceImport(bindings)) {
      names.add(bindings.name.text);
    }
  }
  return names;
}

/** Whether `node` references any of `names`. */
function usesAny(
  node: ts.Node,
  names: ReadonlySet<string>,
  tsModule: typeof ts
): boolean {
  if (names.size === 0) return false;
  let found = false;
  const visit = (child: ts.Node): void => {
    if (found) return;
    if (tsModule.isIdentifier(child) && names.has(child.text)) {
      found = true;
      return;
    }
    tsModule.forEachChild(child, visit);
  };
  visit(node);
  return found;
}

function isReusableDeclaration(stmt: ts.Statement, tsModule: typeof ts): boolean {
  return (
    tsModule.isVariableStatement(stmt) ||
    tsModule.isFunctionDeclaration(stmt) ||
    tsModule.isClassDeclaration(stmt) ||
    tsModule.isEnumDeclaration(stmt) ||
    tsModule.isInterfaceDeclaration(stmt) ||
    tsModule.isTypeAliasDeclaration(stmt)
  );
}

function rewriteImport(
  stmt: ts.ImportDeclaration,
  sourceFile: ts.SourceFile,
  tsModule: typeof ts,
  compilerOptions: ts.CompilerOptions
): string {
  const spec = stmt.moduleSpecifier;
  if (!tsModule.isStringLiteral(spec)) {
    return stmt.getText(sourceFile);
  }
  const resolved = tsModule.resolveModuleName(
    spec.text,
    sourceFile.fileName,
    compilerOptions,
    tsModule.sys
  ).resolvedModule?.resolvedFileName;
  if (!resolved) return stmt.getText(sourceFile);
  if (resolved.includes('/node_modules/') || resolved.endsWith('.d.ts')) {
    return stmt.getText(sourceFile);
  }

  const text = stmt.getText(sourceFile);
  // pathToFileURL produces a forward-slash percent-encoded file:// URL (Windows-safe);
  // JSON.stringify is the correct way to embed it as a JS string literal — not double-escaping.
  return text.replace(
    spec.getText(sourceFile),
    JSON.stringify(pathToFileURL(resolved).href)
  );
}

function sourceMapForExpression(
  expr: ts.Expression,
  sourceFile: ts.SourceFile,
  tsModule: typeof ts
): Record<string, SourceMapEntry> {
  const objectArg = findZObjectCall(expr, tsModule);
  if (objectArg) return buildSourceMapFromObjectLiteral(objectArg, sourceFile, tsModule);
  return buildRootSourceMap(expr, sourceFile);
}

function sourceMapForTarget(
  expr: ts.Expression,
  sourceFile: ts.SourceFile,
  checker: ts.TypeChecker,
  tsModule: typeof ts
): Record<string, SourceMapEntry> {
  if (tsModule.isIdentifier(expr)) {
    const decl = resolveVariableDeclaration(expr, checker, tsModule);
    if (decl?.initializer) {
      return sourceMapForExpression(
        decl.initializer,
        decl.getSourceFile(),
        tsModule
      );
    }
  }

  return sourceMapForExpression(expr, sourceFile, tsModule);
}

function safeName(name: string): string {
  return name.replace(/[^a-zA-Z0-9_]/g, '_');
}
