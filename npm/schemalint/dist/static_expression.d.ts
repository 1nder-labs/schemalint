import type * as ts from 'typescript';
export declare function resolveVariableDeclaration(id: ts.Identifier, checker: ts.TypeChecker, tsModule: typeof ts): ts.VariableDeclaration | undefined;
export declare function unambiguousExpression(expr: ts.Expression | undefined, checker: ts.TypeChecker, tsModule: typeof ts): ts.Expression | undefined;
export declare function sameStaticExpression(left: ts.Expression, right: ts.Expression, checker: ts.TypeChecker, tsModule: typeof ts): boolean;
//# sourceMappingURL=static_expression.d.ts.map