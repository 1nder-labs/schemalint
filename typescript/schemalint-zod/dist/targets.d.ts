import type * as ts from 'typescript';
import type { SourceMapEntry } from './discover.js';
export interface SchemaTarget {
    name: string;
    filePath: string;
    exportName: string;
    sourceMap: Record<string, SourceMapEntry>;
    syntheticSource?: string;
}
export declare function findSchemaTargets(program: ts.Program, fileSet: ReadonlySet<string>, tsModule: typeof ts, compilerOptions: ts.CompilerOptions): SchemaTarget[];
//# sourceMappingURL=targets.d.ts.map