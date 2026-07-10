import type * as ts from 'typescript';

export function resolveVariableDeclaration(
  id: ts.Identifier,
  checker: ts.TypeChecker,
  tsModule: typeof ts
): ts.VariableDeclaration | undefined {
  const symbol = checker.getSymbolAtLocation(id);
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
  return resolveUnambiguous(expr, checker, tsModule, new Set());
}

function resolveUnambiguous(
  expr: ts.Expression | undefined,
  checker: ts.TypeChecker,
  tsModule: typeof ts,
  seen: Set<ts.VariableDeclaration>
): ts.Expression | undefined {
  if (!expr) return undefined;
  const unwrapped = skipParens(expr, tsModule);

  if (tsModule.isConditionalExpression(unwrapped)) {
    const whenTrue = resolveUnambiguous(
      unwrapped.whenTrue,
      checker,
      tsModule,
      seen
    );
    const whenFalse = resolveUnambiguous(
      unwrapped.whenFalse,
      checker,
      tsModule,
      seen
    );
    return whenTrue &&
      whenFalse &&
      sameStaticExpression(whenTrue, whenFalse, checker, tsModule)
      ? whenTrue
      : undefined;
  }

  if (tsModule.isIdentifier(unwrapped)) {
    const declaration = resolveVariableDeclaration(unwrapped, checker, tsModule);
    const initializer = declaration?.initializer;
    const value = initializer && skipParens(initializer, tsModule);
    if (
      declaration &&
      value &&
      (tsModule.isConditionalExpression(value) || tsModule.isIdentifier(value))
    ) {
      if (seen.has(declaration)) return undefined;
      seen.add(declaration);
      const resolved = resolveUnambiguous(value, checker, tsModule, seen);
      seen.delete(declaration);
      if (!resolved) return undefined;
    }
  }

  return unwrapped;
}

export function sameStaticExpression(
  left: ts.Expression,
  right: ts.Expression,
  checker: ts.TypeChecker,
  tsModule: typeof ts
): boolean {
  const a = skipParens(left, tsModule);
  const b = skipParens(right, tsModule);
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

function skipParens(node: ts.Expression, tsModule: typeof ts): ts.Expression {
  while (tsModule.isParenthesizedExpression(node)) node = node.expression;
  return node;
}
