/**
 * Normalize a file-system path to forward slashes.
 *
 * `path.relative()` returns backslash-separated paths on Windows
 * (e.g. `src\\foo.ts`), but picomatch globs use forward slashes.
 * Replacing the OS separator with `/` is a no-op on POSIX and correct
 * on Windows, making the glob filter work on both platforms.
 *
 * The `sep` parameter exists solely for unit-testing Windows paths on a
 * POSIX machine — pass `'\\'` to simulate Windows `path.sep`.
 */
export declare function toPosixPath(p: string, sep?: string): string;
export interface SourceMapEntry {
    file: string;
    line?: number;
}
export interface TargetSpan {
    file: string;
    line: number;
    col: number;
}
/**
 * The wire shape the Rust CLI reads. `provider`, `envelope`, and
 * `canonical_kind` are constant: discovery no longer knows which SDK a
 * schema reaches, so the profile comes from `--profile` or package.json.
 */
export interface DiscoveredModel {
    name: string;
    module_path: string;
    schema: Record<string, unknown>;
    source_map: Record<string, SourceMapEntry>;
    canonical_kind: 'zod.export';
    provider: {
        certainty: 'ambiguous';
    };
    envelope: Record<string, never>;
    usage_span: TargetSpan;
}
export interface DiscoveryWarning {
    model: string;
    message: string;
}
export interface DiscoveryFailure {
    kind: 'evaluation';
    target: string;
    message: string;
}
export interface DiscoveryCounts {
    attempted: number;
    excluded: number;
    discovered: number;
    failed: number;
}
export interface DiscoverResponse {
    models: DiscoveredModel[];
    warnings: DiscoveryWarning[];
    failures: DiscoveryFailure[];
    counts: DiscoveryCounts;
}
/**
 * Discover Zod schemas by walking TypeScript ASTs.
 *
 * 1. Lists the TypeScript files the source scope names, from disk.
 * 2. Groups them by nearest tsconfig.json and builds one program per group;
 *    files with no tsconfig above them share a default program.
 * 3. Finds every Zod object schema those files build (see `findSchemaRoots`).
 * 4. Slices each out of its declaring file and converts it to JSON Schema.
 */
export declare function discoverZodSchemas(sourceGlob: string, exclusions?: string[]): Promise<DiscoverResponse>;
//# sourceMappingURL=discover.d.ts.map