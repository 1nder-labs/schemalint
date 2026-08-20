import type * as ts from 'typescript';
import type { EnvelopeField, ProviderResolution, TargetSpan } from './sdk_adapters.js';
export interface TargetMetadata {
    adapterModule: string;
    canonicalKind: string;
    provider: ProviderResolution;
    envelope: Record<string, EnvelopeField>;
    usageSpan: TargetSpan;
}
export interface TargetExpression {
    name: string;
    sourceFile: ts.SourceFile;
    expression: ts.Expression;
    metadata: TargetMetadata;
}
export interface CarrierExpression {
    api: string;
    fn: ts.FunctionLikeDeclaration;
    paramIndex: number;
    /**
     * Property to read off the call argument. Undefined when the parameter
     * *is* the schema (`f(schema)`), in which case the argument is used whole.
     */
    propertyName?: string;
    explicitName?: string;
    metadata: TargetMetadata;
}
export declare function pushExpressionOrCarrier(targets: TargetExpression[], carriers: CarrierExpression[], api: string, expression: ts.Expression, sourceFile: ts.SourceFile, tsModule: typeof ts, explicitName: string | undefined, metadata: TargetMetadata): void;
export declare function collectCarrierTargets(program: ts.Program, fileSet: ReadonlySet<string>, checker: ts.TypeChecker, tsModule: typeof ts, carriers: CarrierExpression[]): TargetExpression[];
export declare function spanFor(node: ts.Node, sourceFile: ts.SourceFile): TargetSpan;
export declare function stringValueFromExpression(expr: ts.Expression | undefined, checker: ts.TypeChecker, tsModule: typeof ts): string | undefined;
//# sourceMappingURL=target_resolution.d.ts.map