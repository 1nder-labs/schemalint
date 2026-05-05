interface SourceMapEntry {
    file: string;
    line?: number;
}
export interface DiscoveredModel {
    name: string;
    module_path: string;
    schema: Record<string, unknown>;
    source_map: Record<string, SourceMapEntry>;
}
export interface DiscoveryWarning {
    model: string;
    message: string;
}
export interface DiscoverResponse {
    models: DiscoveredModel[];
    warnings: DiscoveryWarning[];
    provider_hint?: string;
}
/**
 * Discover Zod schemas by walking TypeScript ASTs.
 *
 * 1. Reads tsconfig.json to resolve the project file list.
 * 2. Filters files against the user-supplied source glob.
 * 3. Walks each source file's AST looking for `z.object({...})` calls.
 * 4. Extracts property source locations for source map.
 * 5. Dynamically imports each file and evaluates schemas at runtime.
 * 6. Converts schemas to JSON Schema via zod-to-json-schema or native.
 */
export declare function discoverZodSchemas(sourceGlob: string): Promise<DiscoverResponse>;
export {};
//# sourceMappingURL=discover.d.ts.map