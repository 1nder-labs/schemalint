import type * as ts from 'typescript';
import { type TargetExpression, type TargetMetadata } from './target_resolution.js';
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
/**
 * Resolves carriers to their call-site targets, following wrapper chains
 * multiple hops deep: a call site that forwards the schema through one of
 * its own parameters (bare, destructured, or re-spread) yields a new carrier
 * for its enclosing function rather than a dead end, and that carrier is fed
 * back in until no more forwarding is found or the hop limit is hit.
 */
export declare function collectCarrierTargets(program: ts.Program, fileSet: ReadonlySet<string>, checker: ts.TypeChecker, tsModule: typeof ts, carriers: CarrierExpression[]): TargetExpression[];
//# sourceMappingURL=carrier.d.ts.map