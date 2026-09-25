import type * as ts from 'typescript';
import { type SchemaTarget } from './target_emit.js';
export type { SchemaTarget } from './target_emit.js';
export interface TargetFailure {
    kind: 'metadata';
    target: string;
    message: string;
}
export interface TargetDiscovery {
    targets: SchemaTarget[];
    failures: TargetFailure[];
    /** Selected files that import a known provider SDK module. */
    sdkFiles: number;
}
export declare function findSchemaTargets(program: ts.Program, fileSet: ReadonlySet<string>, tsModule: typeof ts, compilerOptions: ts.CompilerOptions): TargetDiscovery;
//# sourceMappingURL=targets.d.ts.map