import type * as ts from 'typescript';

import {
  distinctExpressions,
  staticAlternatives,
  unambiguousExpression,
  unwrapExpression,
} from './static_expression.js';

export function propertyFromExpression(
  expr: ts.Expression | undefined,
  name: string,
  checker: ts.TypeChecker,
  tsModule: typeof ts
): ts.Expression | undefined {
  const candidates: ts.Expression[] = [];
  const containers = staticAlternatives(expr, checker, tsModule);
  if (containers.length === 0) return undefined;
  for (const container of containers) {
    if (!tsModule.isObjectLiteralExpression(container)) return undefined;
    const property = propertyFromObject(container, name, checker, tsModule);
    const stable = unambiguousExpression(property, checker, tsModule);
    if (!stable) return undefined;
    candidates.push(stable);
  }
  const distinct = distinctExpressions(candidates, checker, tsModule);
  return distinct.length === 1 ? distinct[0] : undefined;
}

export function stringPropertyFromExpression(
  expr: ts.Expression | undefined,
  name: string,
  checker: ts.TypeChecker,
  tsModule: typeof ts
): string | undefined {
  const value = propertyFromExpression(expr, name, checker, tsModule);
  return stringLiteralText(value, tsModule);
}

export function propertyFromObject(
  obj: ts.ObjectLiteralExpression,
  name: string,
  checker: ts.TypeChecker,
  tsModule: typeof ts
): ts.Expression | undefined {
  for (const prop of [...obj.properties].reverse()) {
    if (tsModule.isPropertyAssignment(prop)) {
      if (propertyName(prop.name, tsModule) === name) return prop.initializer;
      continue;
    }

    // `{ schema }` — the value is the name itself.
    if (tsModule.isShorthandPropertyAssignment(prop)) {
      if (prop.name.text === name) return prop.name;
      continue;
    }

    if (tsModule.isSpreadAssignment(prop)) {
      const fromSpread = propertyFromExpression(
        prop.expression,
        name,
        checker,
        tsModule
      );
      if (fromSpread) return fromSpread;
    }
  }
  return undefined;
}

export function stringLiteralText(
  expr: ts.Expression | undefined,
  tsModule: typeof ts
): string | undefined {
  return expr && tsModule.isStringLiteralLike(expr) ? expr.text : undefined;
}

export function propertyName(
  name: ts.PropertyName,
  tsModule: typeof ts
): string | undefined {
  if (tsModule.isIdentifier(name) || tsModule.isStringLiteral(name)) {
    return name.text;
  }
  return undefined;
}
