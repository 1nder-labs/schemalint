import type * as ts from 'typescript';
export interface TargetExpression {
    name: string;
    sourceFile: ts.SourceFile;
    expression: ts.Expression;
}
export interface CarrierExpression {
    api: string;
    fn: ts.FunctionLikeDeclaration;
    paramName: string;
    propertyName: string;
    explicitName?: string;
}
export declare function pushExpressionOrCarrier(targets: TargetExpression[], carriers: CarrierExpression[], api: string, expression: ts.Expression, sourceFile: ts.SourceFile, tsModule: typeof ts, explicitName?: string): void;
export declare function collectCarrierTargets(program: ts.Program, fileSet: ReadonlySet<string>, checker: ts.TypeChecker, tsModule: typeof ts, carriers: CarrierExpression[]): TargetExpression[];
export declare function objectExpression(expr: ts.Expression | undefined, checker: ts.TypeChecker, tsModule: typeof ts): ts.ObjectLiteralExpression | undefined;
export declare function propertyFromExpression(expr: ts.Expression | undefined, name: string, checker: ts.TypeChecker, tsModule: typeof ts): ts.Expression | undefined;
export declare function stringPropertyFromExpression(expr: ts.Expression | undefined, name: string, checker: ts.TypeChecker, tsModule: typeof ts): string | undefined;
export declare function resolveVariableDeclaration(id: ts.Identifier, checker: ts.TypeChecker, tsModule: typeof ts): ts.VariableDeclaration | undefined;
//# sourceMappingURL=target_resolution.d.ts.map