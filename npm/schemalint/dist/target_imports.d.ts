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
export declare function resolveTargetAdapter(expression: ts.Expression, imports: TargetImports, tsModule: typeof ts): SdkAdapter | undefined;
export {};
//# sourceMappingURL=target_imports.d.ts.map