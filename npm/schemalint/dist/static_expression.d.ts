import type * as ts from 'typescript';
export declare function resolveVariableDeclaration(id: ts.Identifier, checker: ts.TypeChecker, tsModule: typeof ts): ts.VariableDeclaration | undefined;
export declare function unambiguousExpression(expr: ts.Expression | undefined, checker: ts.TypeChecker, tsModule: typeof ts): ts.Expression | undefined;
export declare function staticAlternatives(expr: ts.Expression | undefined, checker: ts.TypeChecker, tsModule: typeof ts): ts.Expression[];
export declare function distinctExpressions(expressions: readonly ts.Expression[], checker: ts.TypeChecker, tsModule: typeof ts): ts.Expression[];
export declare function sameStaticExpression(left: ts.Expression, right: ts.Expression, checker: ts.TypeChecker, tsModule: typeof ts): boolean;
export declare function unwrapExpression(node: ts.Expression, tsModule: typeof ts): ts.Expression;
//# sourceMappingURL=static_expression.d.ts.map