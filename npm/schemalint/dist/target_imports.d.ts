import type * as ts from 'typescript';
import { type SdkAdapter } from './sdk_adapters.js';
interface ImportedObject {
    module: string;
    exportPath: string;
}
export interface TargetImports {
    functions: Map<string, SdkAdapter>;
    objects: Map<string, ImportedObject>;
    namespaces: Map<string, string>;
}
export declare function collectTargetImports(sourceFile: ts.SourceFile, tsModule: typeof ts): TargetImports;
/** Whether `sourceFile` imports anything from a known provider SDK module. */
export declare function importsAdapterModule(sourceFile: ts.SourceFile, tsModule: typeof ts): boolean;
export declare function resolveTargetAdapter(expression: ts.Expression, imports: TargetImports, tsModule: typeof ts): SdkAdapter | undefined;
export {};
//# sourceMappingURL=target_imports.d.ts.map