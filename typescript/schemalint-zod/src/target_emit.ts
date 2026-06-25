import { pathToFileURL } from 'node:url';

import type * as ts from 'typescript';

import type { SourceMapEntry } from './discover.js';
import {
  buildRootSourceMap,
  buildSourceMapFromObjectLiteral,
  findZObjectCall,
  hasExportModifier,
} from './discover_ast.js';
import {
  resolveVariableDeclaration,
  type TargetExpression,
} from './target_resolution.js';

export interface SchemaTarget {
  name: string;
  filePath: string;
  exportName: string;
  sourceMap: Record<string, SourceMapEntry>;
  syntheticSource?: string;
}

export function resolveTarget(
  target: TargetExpression,
  checker: ts.TypeChecker,
  tsModule: typeof ts,
  compilerOptions: ts.CompilerOptions
): SchemaTarget {
  const expr = skipParens(target.expression, tsModule);
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
      };
    }
  }

  const exportName = `__schemalint_target_${safeName(target.name)}`;
  return {
    name: target.name,
    filePath: sourceFile.fileName,
    exportName,
    sourceMap,
    syntheticSource: buildSyntheticModule(
      sourceFile,
      expr,
      exportName,
      tsModule,
      compilerOptions
    ),
  };
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
  compilerOptions: ts.CompilerOptions
): string {
  const parts: string[] = [];
  // Identify the top-level statement that directly contains the target
  // expression so we can skip it (we replace it with the export below).
  const containingStmt = sourceFile.statements.find(
    (stmt) =>
      stmt.getStart(sourceFile) <= expr.getStart(sourceFile) &&
      expr.getEnd() <= stmt.getEnd()
  );

  for (const stmt of sourceFile.statements) {
    if (tsModule.isImportDeclaration(stmt)) {
      parts.push(rewriteImport(stmt, sourceFile, tsModule, compilerOptions));
      continue;
    }
    // Include all reusable declarations except the one containing the target
    // expression. This allows the target to reference helpers declared either
    // before or after it in source order without a ReferenceError.
    if (stmt !== containingStmt && isReusableDeclaration(stmt, tsModule)) {
      parts.push(stmt.getText(sourceFile));
    }
  }
  parts.push(`export const ${exportName} = ${expr.getText(sourceFile)};`);
  return parts.join('\n\n');
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

function skipParens(node: ts.Expression, tsModule: typeof ts): ts.Expression {
  while (tsModule.isParenthesizedExpression(node)) node = node.expression;
  return node;
}

function safeName(name: string): string {
  return name.replace(/[^a-zA-Z0-9_]/g, '_');
}
