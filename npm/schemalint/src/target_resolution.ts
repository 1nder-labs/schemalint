import type * as ts from 'typescript';

import type {
  EnvelopeField,
  ProviderResolution,
  TargetSpan,
} from './sdk_adapters.js';
import {
  resolveVariableDeclaration,
  sameStaticExpression,
  unambiguousExpression,
} from './static_expression.js';

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
  paramName: string;
  propertyName: string;
  explicitName?: string;
  metadata: TargetMetadata;
}

export function pushExpressionOrCarrier(
  targets: TargetExpression[],
  carriers: CarrierExpression[],
  api: string,
  expression: ts.Expression,
  sourceFile: ts.SourceFile,
  tsModule: typeof ts,
  explicitName: string | undefined,
  metadata: TargetMetadata
): void {
  const carrier = carrierExpression(
    api,
    expression,
    tsModule,
    explicitName,
    metadata
  );
  if (carrier) {
    carriers.push(carrier);
    return;
  }

  targets.push(
    namedTarget(api, expression, sourceFile, tsModule, explicitName, metadata)
  );
}

export function collectCarrierTargets(
  program: ts.Program,
  fileSet: ReadonlySet<string>,
  checker: ts.TypeChecker,
  tsModule: typeof ts,
  carriers: CarrierExpression[]
): TargetExpression[] {
  if (carriers.length === 0) return [];

  const targets: TargetExpression[] = [];
  for (const sourceFile of program.getSourceFiles()) {
    if (
      sourceFile.isDeclarationFile ||
      sourceFile.fileName.includes('node_modules') ||
      !fileSet.has(sourceFile.fileName)
    ) {
      continue;
    }

    function walk(node: ts.Node): void {
      if (tsModule.isCallExpression(node)) {
        for (const carrier of carriers) {
          const target = carrierTargetFromCall(
            node,
            sourceFile,
            checker,
            tsModule,
            carrier
          );
          if (target) targets.push(target);
        }
      }
      tsModule.forEachChild(node, walk);
    }

    tsModule.forEachChild(sourceFile, walk);
  }

  return targets;
}

export function propertyFromExpression(
  expr: ts.Expression | undefined,
  name: string,
  checker: ts.TypeChecker,
  tsModule: typeof ts
): ts.Expression | undefined {
  if (!expr) return undefined;
  const unwrapped = skipParens(expr, tsModule);

  if (tsModule.isObjectLiteralExpression(unwrapped)) {
    return unambiguousExpression(
      propertyFromObject(unwrapped, name, checker, tsModule),
      checker,
      tsModule
    );
  }

  if (tsModule.isIdentifier(unwrapped)) {
    const decl = resolveVariableDeclaration(unwrapped, checker, tsModule);
    return propertyFromExpression(decl?.initializer, name, checker, tsModule);
  }

  if (tsModule.isConditionalExpression(unwrapped)) {
    const whenTrue = propertyFromExpression(
      unwrapped.whenTrue,
      name,
      checker,
      tsModule
    );
    const whenFalse = propertyFromExpression(
      unwrapped.whenFalse,
      name,
      checker,
      tsModule
    );
    return whenTrue &&
      whenFalse &&
      sameStaticExpression(whenTrue, whenFalse, checker, tsModule)
      ? whenTrue
      : undefined;
  }

  return undefined;
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

function carrierExpression(
  api: string,
  expression: ts.Expression,
  tsModule: typeof ts,
  explicitName: string | undefined,
  metadata: TargetMetadata
): CarrierExpression | undefined {
  const expr = skipParens(expression, tsModule);
  if (!tsModule.isPropertyAccessExpression(expr)) return undefined;
  if (!tsModule.isIdentifier(expr.expression)) return undefined;

  const paramName = expr.expression.text;
  const fn = enclosingCarrierFunction(expr, paramName, tsModule);
  if (!fn) return undefined;

  return {
    api,
    fn,
    paramName,
    propertyName: expr.name.text,
    explicitName,
    metadata,
  };
}

function enclosingCarrierFunction(
  node: ts.Node,
  paramName: string,
  tsModule: typeof ts
): ts.FunctionLikeDeclaration | undefined {
  let current = node.parent;
  while (current) {
    if (
      tsModule.isFunctionDeclaration(current) ||
      tsModule.isFunctionExpression(current) ||
      tsModule.isArrowFunction(current) ||
      tsModule.isMethodDeclaration(current)
    ) {
      const hasParam = current.parameters.some(
        (param) =>
          tsModule.isIdentifier(param.name) && param.name.text === paramName
      );
      if (hasParam) return current;
    }
    current = current.parent;
  }
  return undefined;
}

function carrierTargetFromCall(
  call: ts.CallExpression,
  sourceFile: ts.SourceFile,
  checker: ts.TypeChecker,
  tsModule: typeof ts,
  carrier: CarrierExpression
): TargetExpression | undefined {
  if (!sameSymbol(call.expression, carrier.fn, checker, tsModule)) {
    return undefined;
  }

  const paramIndex = carrier.fn.parameters.findIndex(
    (param) =>
      tsModule.isIdentifier(param.name) && param.name.text === carrier.paramName
  );
  if (paramIndex === -1) return undefined;

  const schema = propertyFromExpression(
    call.arguments[paramIndex],
    carrier.propertyName,
    checker,
    tsModule
  );
  if (!schema) return undefined;

  const name =
    carrier.explicitName ??
    stringPropertyFromExpression(
      call.arguments[paramIndex],
      'name',
      checker,
      tsModule
    );
  return namedTarget(carrier.api, schema, sourceFile, tsModule, name, {
    ...carrier.metadata,
    usageSpan: spanFor(call, sourceFile),
  });
}

function sameSymbol(
  expression: ts.Expression,
  fn: ts.FunctionLikeDeclaration,
  checker: ts.TypeChecker,
  tsModule: typeof ts
): boolean {
  const symbol = checker.getSymbolAtLocation(expression);
  const aliased =
    symbol && (symbol.flags & tsModule.SymbolFlags.Alias)
      ? checker.getAliasedSymbol(symbol)
      : symbol;
  return aliased?.declarations?.some((decl) => decl === fn) ?? false;
}

function propertyFromObject(
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

function namedTarget(
  api: string,
  expression: ts.Expression,
  sourceFile: ts.SourceFile,
  tsModule: typeof ts,
  explicitName: string | undefined,
  metadata: TargetMetadata
): TargetExpression {
  const { line } = sourceFile.getLineAndCharacterOfPosition(
    expression.getStart(sourceFile)
  );
  const expr = skipParens(expression, tsModule);
  const suffix =
    explicitName ??
    (tsModule.isIdentifier(expr) ? expr.text : `inline:${line + 1}`);
  return {
    name: `${api}:${suffix}`,
    sourceFile,
    expression,
    metadata,
  };
}

export function spanFor(node: ts.Node, sourceFile: ts.SourceFile): TargetSpan {
  const { line, character } = sourceFile.getLineAndCharacterOfPosition(
    node.getStart(sourceFile)
  );
  return { file: sourceFile.fileName, line: line + 1, col: character + 1 };
}

export function stringValueFromExpression(
  expr: ts.Expression | undefined,
  checker: ts.TypeChecker,
  tsModule: typeof ts
): string | undefined {
  if (!expr) return undefined;
  const unwrapped = skipParens(expr, tsModule);
  const literal = stringLiteralText(unwrapped, tsModule);
  if (literal !== undefined) return literal;
  if (tsModule.isIdentifier(unwrapped)) {
    const decl = resolveVariableDeclaration(unwrapped, checker, tsModule);
    return stringValueFromExpression(decl?.initializer, checker, tsModule);
  }
  if (tsModule.isConditionalExpression(unwrapped)) {
    const whenTrue = stringValueFromExpression(
      unwrapped.whenTrue,
      checker,
      tsModule
    );
    const whenFalse = stringValueFromExpression(
      unwrapped.whenFalse,
      checker,
      tsModule
    );
    return whenTrue !== undefined && whenTrue === whenFalse
      ? whenTrue
      : undefined;
  }
  return undefined;
}

function stringLiteralText(
  expr: ts.Expression | undefined,
  tsModule: typeof ts
): string | undefined {
  return expr && tsModule.isStringLiteralLike(expr) ? expr.text : undefined;
}

function propertyName(
  name: ts.PropertyName,
  tsModule: typeof ts
): string | undefined {
  if (tsModule.isIdentifier(name) || tsModule.isStringLiteral(name)) {
    return name.text;
  }
  return undefined;
}

function skipParens(node: ts.Expression, tsModule: typeof ts): ts.Expression {
  while (tsModule.isParenthesizedExpression(node)) node = node.expression;
  return node;
}
