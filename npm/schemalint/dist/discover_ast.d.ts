import type * as ts from 'typescript';
import type { SourceMapEntry } from './discover.js';
export interface ExportedSchemaCall {
    name: string;
    objectArg: ts.ObjectLiteralExpression;
}
/**
 * Find top-level `const` declarations that are `z.object({...})` calls.
 */
export declare function findExportedSchemaCalls(sourceFile: ts.SourceFile, tsModule: typeof ts): ExportedSchemaCall[];
/**
 * Scan a source file's import declarations for provider SDKs.
 * Returns "openai" or "anthropic" if detected, undefined otherwise.
 */
export declare function scanProviderImports(sourceFile: ts.SourceFile, tsModule: typeof ts): string | undefined;
export declare function hasExportModifier(node: ts.Node, tsModule: typeof ts): boolean;
/**
 * Given a node, if it is `z.object({...})`, return the ObjectLiteralExpression argument.
 * Handles chaining: `z.object({...}).extend({...})` — returns the initial object.
 */
export declare function findZObjectCall(node: ts.Node, tsModule: typeof ts): ts.ObjectLiteralExpression | null;
/**
 * Walk an ObjectLiteralExpression (`{ email: z.string(), ... }`) and build a
 * source map mapping JSON Pointer paths to file:line locations.
 *
 * Handles nested `z.object({...})` by recursing into inner object literals.
 */
export declare function buildSourceMapFromObjectLiteral(objLit: ts.ObjectLiteralExpression, sourceFile: ts.SourceFile, tsModule: typeof ts): Record<string, SourceMapEntry>;
export declare function buildRootSourceMap(node: ts.Node, sourceFile: ts.SourceFile): Record<string, SourceMapEntry>;
//# sourceMappingURL=discover_ast.d.ts.map