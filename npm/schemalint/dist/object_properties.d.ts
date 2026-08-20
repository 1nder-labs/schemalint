import type * as ts from 'typescript';
export declare function propertyFromExpression(expr: ts.Expression | undefined, name: string, checker: ts.TypeChecker, tsModule: typeof ts): ts.Expression | undefined;
export declare function stringPropertyFromExpression(expr: ts.Expression | undefined, name: string, checker: ts.TypeChecker, tsModule: typeof ts): string | undefined;
export declare function propertyFromObject(obj: ts.ObjectLiteralExpression, name: string, checker: ts.TypeChecker, tsModule: typeof ts): ts.Expression | undefined;
export declare function stringLiteralText(expr: ts.Expression | undefined, tsModule: typeof ts): string | undefined;
export declare function propertyName(name: ts.PropertyName, tsModule: typeof ts): string | undefined;
//# sourceMappingURL=object_properties.d.ts.map