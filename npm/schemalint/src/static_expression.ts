import type * as ts from 'typescript';

export function resolveVariableDeclaration(
  id: ts.Identifier,
  checker: ts.TypeChecker,
  tsModule: typeof ts
): ts.VariableDeclaration | undefined {
  // In `{ schema }` the identifier's own symbol is the object literal's
  // property, not the value it stands for — the checker exposes the value
  // through a dedicated lookup.
  const symbol = tsModule.isShorthandPropertyAssignment(id.parent)
    ? checker.getShorthandAssignmentValueSymbol(id.parent)
    : checker.getSymbolAtLocation(id);
  const aliased =
    symbol && (symbol.flags & tsModule.SymbolFlags.Alias)
      ? checker.getAliasedSymbol(symbol)
      : symbol;
  const decl = aliased?.valueDeclaration ?? aliased?.declarations?.[0];
  return decl && tsModule.isVariableDeclaration(decl) ? decl : undefined;
}

export function unambiguousExpression(
  expr: ts.Expression | undefined,
  checker: ts.TypeChecker,
  tsModule: typeof ts
): ts.Expression | undefined {
  if (!expr) return undefined;
  const unwrapped = unwrapExpression(expr, tsModule);
  const alternatives = staticAlternatives(unwrapped, checker, tsModule);
  if (alternatives.length !== 1) return undefined;
  return tsModule.isConditionalExpression(unwrapped)
    ? alternatives[0]
    : unwrapped;
}

export function staticAlternatives(
  expr: ts.Expression | undefined,
  checker: ts.TypeChecker,
  tsModule: typeof ts
): ts.Expression[] {
  return resolveAlternatives(expr, checker, tsModule, new Set());
}

function resolveAlternatives(
  expr: ts.Expression | undefined,
  checker: ts.TypeChecker,
  tsModule: typeof ts,
  seen: Set<ts.VariableDeclaration>
): ts.Expression[] {
  if (!expr) return [];
  const unwrapped = unwrapExpression(expr, tsModule);

  if (tsModule.isConditionalExpression(unwrapped)) {
    return distinctExpressions(
      [unwrapped.whenTrue, unwrapped.whenFalse].flatMap((branch) =>
        resolveAlternatives(branch, checker, tsModule, seen)
      ),
      checker,
      tsModule
    );
  }

  if (tsModule.isIdentifier(unwrapped)) {
    const declaration = resolveVariableDeclaration(unwrapped, checker, tsModule);
    const initializer = declaration?.initializer;
    if (declaration && initializer) {
      if (seen.has(declaration)) return [];
      seen.add(declaration);
      const resolved = resolveAlternatives(initializer, checker, tsModule, seen);
      seen.delete(declaration);
      return resolved;
    }
  }

  return [unwrapped];
}

export function distinctExpressions(
  expressions: readonly ts.Expression[],
  checker: ts.TypeChecker,
  tsModule: typeof ts
): ts.Expression[] {
  const distinct: ts.Expression[] = [];
  for (const expression of expressions) {
    if (
      !distinct.some((candidate) =>
        sameStaticExpression(candidate, expression, checker, tsModule)
      )
    ) {
      distinct.push(expression);
    }
  }
  return distinct;
}

export function sameStaticExpression(
  left: ts.Expression,
  right: ts.Expression,
  checker: ts.TypeChecker,
  tsModule: typeof ts
): boolean {
  const a = unwrapExpression(left, tsModule);
  const b = unwrapExpression(right, tsModule);
  if (a === b) return true;
  if (tsModule.isStringLiteralLike(a) && tsModule.isStringLiteralLike(b)) {
    return a.text === b.text;
  }
  if (tsModule.isIdentifier(a) && tsModule.isIdentifier(b)) {
    const symbol = checker.getSymbolAtLocation(a);
    return symbol !== undefined && symbol === checker.getSymbolAtLocation(b);
  }
  return false;
}

export function unwrapExpression(
  node: ts.Expression,
  tsModule: typeof ts
): ts.Expression {
  while (tsModule.isParenthesizedExpression(node)) node = node.expression;
  return node;
}
