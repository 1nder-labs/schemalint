import type * as ts from 'typescript';
export interface TargetImports {
    outputCalls: Set<string>;
    toolCalls: Set<string>;
    formatCalls: Set<string>;
    outputObjects: Set<string>;
    aiNamespaces: Set<string>;
    helperNamespaces: Set<string>;
}
export interface CallRef {
    name: string;
    receiver?: string;
}
export declare const OUTPUT_CALLS: Set<string>;
export declare const TOOL_CALLS: Set<string>;
export declare const FORMAT_CALLS: Set<string>;
export declare function collectTargetImports(sourceFile: ts.SourceFile, tsModule: typeof ts): TargetImports;
export declare function isProviderCall(ref: CallRef, imports: TargetImports): boolean;
//# sourceMappingURL=target_imports.d.ts.map