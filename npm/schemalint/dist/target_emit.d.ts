import type * as ts from 'typescript';
import type { SourceMapEntry } from './discover.js';
import { type TargetExpression } from './target_resolution.js';
export interface SchemaTarget {
    name: string;
    filePath: string;
    exportName: string;
    sourceMap: Record<string, SourceMapEntry>;
    syntheticSource?: string;
}
export declare function resolveTarget(target: TargetExpression, checker: ts.TypeChecker, tsModule: typeof ts, compilerOptions: ts.CompilerOptions): SchemaTarget;
//# sourceMappingURL=target_emit.d.ts.map