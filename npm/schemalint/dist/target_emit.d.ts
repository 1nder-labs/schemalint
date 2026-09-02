import type * as ts from 'typescript';
import type { SourceMapEntry } from './discover.js';
import type { TargetExpression } from './target_resolution.js';
import type { EnvelopeField, ProviderResolution, TargetSpan } from './sdk_adapters.js';
export interface SchemaTarget {
    name: string;
    filePath: string;
    exportName: string;
    sourceMap: Record<string, SourceMapEntry>;
    canonicalKind: string;
    provider: ProviderResolution;
    envelope: Record<string, EnvelopeField>;
    usageSpan: TargetSpan;
    syntheticSource: string;
    /**
     * `<file>#<name>` of the module-level declaration this target evaluates,
     * when it is one. Lets explicit-scope discovery skip an exported schema
     * already reached through a provider call site.
     */
    declaration?: string;
}
export declare function declarationKey(file: string, name: string): string;
export declare function resolveTarget(target: TargetExpression, checker: ts.TypeChecker, tsModule: typeof ts, compilerOptions: ts.CompilerOptions): SchemaTarget;
//# sourceMappingURL=target_emit.d.ts.map