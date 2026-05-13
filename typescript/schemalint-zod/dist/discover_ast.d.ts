import type * as ts from 'typescript';
import type { SourceMapEntry } from './discover.js';
interface ExportedSchemaCall {
    name: string;
    objectArg: ts.ObjectLiteralExpression;
}
/**
 * Find top-level `const` declarations that are `z.object({...})` calls.
 * If `zodFormatRefs` is provided, non-exported schemas referenced by
 * zodTextFormat/zodResponseFormat are also included.
 */
export declare function findExportedSchemaCalls(sourceFile: ts.SourceFile, tsModule: typeof ts, zodFormatRefs?: Set<string>): ExportedSchemaCall[];
/**
 * Scan for zodTextFormat(MySchema, "name") and zodResponseFormat(MySchema, "name")
 * call expressions. Returns the list of schema names referenced as the first argument.
 */
export declare function scanZodTextFormatRefs(sourceFile: ts.SourceFile, tsModule: typeof ts): string[];
/**
 * Scan a source file's import declarations for provider SDKs.
 * Returns "openai" or "anthropic" if detected, undefined otherwise.
 */
export declare function scanProviderImports(sourceFile: ts.SourceFile, tsModule: typeof ts): string | undefined;
/**
 * Walk an ObjectLiteralExpression (`{ email: z.string(), ... }`) and build a
 * source map mapping JSON Pointer paths to file:line locations.
 *
 * Handles nested `z.object({...})` by recursing into inner object literals.
 */
export declare function buildSourceMapFromObjectLiteral(objLit: ts.ObjectLiteralExpression, sourceFile: ts.SourceFile, tsModule: typeof ts): Record<string, SourceMapEntry>;
export {};
//# sourceMappingURL=discover_ast.d.ts.map