import type picomatch from 'picomatch';
import type * as ts from 'typescript';
import type { SourceScope } from './source_scope.js';
/** One TypeScript program's worth of source files sharing compiler options. */
export interface ProjectGroup {
    /** The nearest tsconfig.json, or undefined for the synthesized default. */
    configPath: string | undefined;
    compilerOptions: ts.CompilerOptions;
    files: string[];
}
/**
 * Every TypeScript source file the scope names, as real paths.
 *
 * Listed from disk rather than from a tsconfig's include list, so a scope can
 * reach packages whose tsconfig is not the one at the working directory.
 * Real paths make `/tmp/x` and `/private/tmp/x` (a macOS symlink) the same
 * file when the program's own module resolution reports the resolved one.
 */
export declare function listScopeFiles(scope: SourceScope, projectRoot: string, tsModule: typeof ts, matcher: typeof picomatch): string[];
/**
 * Group files by their nearest ancestor tsconfig.json, one group per config.
 * Files with no tsconfig above them share a synthesized default group, so a
 * package that is not itself a TypeScript project is still checked.
 */
export declare function groupByProject(files: readonly string[], tsModule: typeof ts, honorExclude: boolean): ProjectGroup[];
//# sourceMappingURL=projects.d.ts.map