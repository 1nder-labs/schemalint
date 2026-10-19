import type * as ts from 'typescript';
import type { SourceMapEntry } from './discover.js';
/** One schema to evaluate: where it is declared and the expression that builds it. */
export interface SchemaRoot {
    name: string;
    /** The declaring file; the schema is sliced and evaluated in its context. */
    sourceFile: ts.SourceFile;
    /** The slice seed: a module-level declaration's name, or the schema expression. */
    seed: ts.Expression;
    sourceMap: Record<string, SourceMapEntry>;
    /** Identifies the schema however many ways it is reached. */
    key: string;
}
/**
 * Every Zod object schema `sourceFiles` can build without running a
 * function — declared or inline, exported or not, at module scope or inside a
 * function body — plus the schemas they re-export from other files.
 *
 * No SDK, wrapper, or spelling is tracked: whatever a schema is later handed
 * to, its value is built here. A schema that uses a parameter, a local, or
 * `this` depends on run-time input, so it cannot be checked statically and is
 * skipped. A schema nested in another is linted as part of its root.
 */
export declare function findSchemaRoots(sourceFiles: readonly ts.SourceFile[], checker: ts.TypeChecker, tsModule: typeof ts): SchemaRoot[];
//# sourceMappingURL=schema_roots.d.ts.map