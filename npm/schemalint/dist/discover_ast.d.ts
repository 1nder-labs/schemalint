import type * as ts from 'typescript';
import type { SourceMapEntry } from './discover.js';
/**
 * The object literal a Zod object schema is built from: the first argument of
 * the innermost call in its chain (`z.object({...}).extend(...)`,
 * `z.strictObject({...}).describe('x')`). Matched by shape, not by name.
 */
export declare function findZObjectCall(node: ts.Node, tsModule: typeof ts): ts.ObjectLiteralExpression | null;
/**
 * Escape a single JSON Pointer (RFC 6901) segment.
 *
 * `~` is replaced with `~0` first, then `/` is replaced with `~1`. The order
 * matters: escaping `/` first would introduce new `~1` sequences that the
 * `~` step would then re-escape, corrupting a name that already contains a
 * literal `~1`.
 */
export declare function escapePointerSegment(segment: string): string;
/**
 * Walk an ObjectLiteralExpression (`{ email: z.string(), ... }`) and build a
 * source map mapping JSON Pointer paths to file:line locations.
 *
 * Handles nested `z.object({...})` by recursing into inner object literals.
 */
export declare function buildSourceMapFromObjectLiteral(objLit: ts.ObjectLiteralExpression, sourceFile: ts.SourceFile, tsModule: typeof ts): Record<string, SourceMapEntry>;
export declare function buildRootSourceMap(node: ts.Node, sourceFile: ts.SourceFile): Record<string, SourceMapEntry>;
//# sourceMappingURL=discover_ast.d.ts.map