import type picomatch from 'picomatch';
export interface SourceScope {
    /** The glob actually matched against project files. */
    pattern: string;
    isMatch: (relativePath: string) => boolean;
    /**
     * The source named a literal file or directory rather than a glob. The
     * user pointed at these files, so every exported schema in them is in
     * scope, not only the ones traced to a provider call.
     */
    explicit: boolean;
}
/**
 * Turn a `--source` value into a file matcher. A literal path to an existing
 * directory expands to every TypeScript file beneath it; a literal path to an
 * existing file matches that file alone; anything else is a glob.
 */
export declare function resolveSourceScope(source: string, matcher: typeof picomatch, projectRoot: string): SourceScope;
//# sourceMappingURL=source_scope.d.ts.map