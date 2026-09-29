import type * as ts from 'typescript';
import type { SchemaTarget } from './targets.js';
/**
 * Every exported `z.object({...})` schema reachable from `sourceFiles`, the
 * fallback discovery source. Re-exports (`export * from`, `export { X } from`)
 * are followed to the declaring file, and a schema reached more than once is
 * reported once.
 */
export declare function exportedSchemaTargets(sourceFiles: readonly ts.SourceFile[], checker: ts.TypeChecker, tsModule: typeof ts, compilerOptions: ts.CompilerOptions): SchemaTarget[];
//# sourceMappingURL=exported_targets.d.ts.map