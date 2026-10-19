import type picomatch from 'picomatch';
export interface SourceScope {
    /** The glob actually matched against project files. */
    pattern: string;
    isMatch: (relativePath: string) => boolean;
    /** Real path of the literal file or directory the source names, if it names one. */
    target?: {
        kind: 'file' | 'directory';
        absolute: string;
    };
}
/**
 * Turn a `--source` value into a file matcher. A literal path to an existing
 * directory expands to every TypeScript file beneath it; a literal path to an
 * existing file matches that file alone; anything else is a glob.
 */
export declare function resolveSourceScope(source: string, matcher: typeof picomatch, projectRoot: string): SourceScope;
//# sourceMappingURL=source_scope.d.ts.map