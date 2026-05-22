import type * as ts from 'typescript';
import { type SchemaTarget } from './target_emit.js';
export type { SchemaTarget } from './target_emit.js';
export declare function findSchemaTargets(program: ts.Program, fileSet: ReadonlySet<string>, tsModule: typeof ts, compilerOptions: ts.CompilerOptions): SchemaTarget[];
//# sourceMappingURL=targets.d.ts.map