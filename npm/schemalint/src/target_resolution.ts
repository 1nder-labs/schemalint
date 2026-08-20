import type * as ts from 'typescript';

export interface TargetExpression {
  name: string;
  sourceFile: ts.SourceFile;
  expression: ts.Expression;
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
}

interface CarrierParam {
  fn: ts.FunctionLikeDeclaration;
  paramIndex: number;
  propertyName?: string;
}

export function pushExpressionOrCarrier(
  targets: TargetExpression[],
  carriers: CarrierExpression[],
  api: string,
  expression: ts.Expression,
  sourceFile: ts.SourceFile,
  tsModule: typeof ts,
  explicitName?: string
): void {
  const carrier = carrierExpression(
    api,
    expression,
    tsModule,
    explicitName
  );
  if (carrier) {
    carriers.push(carrier);
    return;
  }

  targets.push(namedTarget(api, expression, sourceFile, tsModule, explicitName));
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

export function objectExpression(
  expr: ts.Expression | undefined,
  checker: ts.TypeChecker,
  tsModule: typeof ts
): ts.ObjectLiteralExpression | undefined {
  if (!expr) return undefined;
  const unwrapped = skipParens(expr, tsModule);
  if (tsModule.isObjectLiteralExpression(unwrapped)) return unwrapped;

  if (tsModule.isIdentifier(unwrapped)) {
    const decl = resolveVariableDeclaration(unwrapped, checker, tsModule);
    if (decl?.initializer) {
      return objectExpression(decl.initializer, checker, tsModule);
    }
  }

  if (tsModule.isConditionalExpression(unwrapped)) {
    return (
      objectExpression(unwrapped.whenTrue, checker, tsModule) ??
      objectExpression(unwrapped.whenFalse, checker, tsModule)
    );
  }

  return undefined;
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
    return propertyFromObject(unwrapped, name, checker, tsModule);
  }

  if (tsModule.isIdentifier(unwrapped)) {
    const decl = resolveVariableDeclaration(unwrapped, checker, tsModule);
    return propertyFromExpression(decl?.initializer, name, checker, tsModule);
  }

  if (tsModule.isConditionalExpression(unwrapped)) {
    return (
      propertyFromExpression(unwrapped.whenTrue, name, checker, tsModule) ??
      propertyFromExpression(unwrapped.whenFalse, name, checker, tsModule)
    );
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

function carrierExpression(
  api: string,
  expression: ts.Expression,
  tsModule: typeof ts,
  explicitName?: string
): CarrierExpression | undefined {
  const expr = skipParens(expression, tsModule);

  // `opts.schema` — the schema is a property of a whole parameter.
  if (
    tsModule.isPropertyAccessExpression(expr) &&
    tsModule.isIdentifier(expr.expression)
  ) {
    const param = carrierParam(expr, expr.expression.text, tsModule);
    // A destructured binding is already a property read; `opts.schema` on top
    // of one would be a second hop this doesn't follow.
    if (!param || param.propertyName !== undefined) return undefined;
    return { ...param, api, propertyName: expr.name.text, explicitName };
  }

  // A bare `schema` — either the parameter itself (`f(schema)`) or destructured
  // out of a parameter object (`f({ schema })`). Both arrive at the call site
  // inside the same argument; `carrierParam` reports which read recovers it.
  if (tsModule.isIdentifier(expr)) {
    const param = carrierParam(expr, expr.text, tsModule);
    return param && { ...param, api, explicitName };
  }

  return undefined;
}

/**
 * Locate the enclosing function that supplies `name` as a parameter, and
 * report how to recover the matching argument at a call site: the parameter
 * index, plus the property to read off it when the parameter was destructured
 * (`f({ schema })`, or renamed as `f({ schema: s })`).
 *
 * ponytail: resolves by walking parent scopes, not via the checker, so a local
 * that shadows a parameter of the same name is misread as that parameter.
 * Switch to `checker.getSymbolAtLocation` if a real codebase ever shadows one.
 */
function carrierParam(
  node: ts.Node,
  name: string,
  tsModule: typeof ts
): CarrierParam | undefined {
  for (let current = node.parent; current; current = current.parent) {
    if (
      !tsModule.isFunctionDeclaration(current) &&
      !tsModule.isFunctionExpression(current) &&
      !tsModule.isArrowFunction(current) &&
      !tsModule.isMethodDeclaration(current)
    ) {
      continue;
    }

    const fn: ts.FunctionLikeDeclaration = current;
    for (let index = 0; index < fn.parameters.length; index++) {
      const bound = fn.parameters[index].name;

      if (tsModule.isIdentifier(bound)) {
        if (bound.text === name) return { fn, paramIndex: index };
        continue;
      }

      if (!tsModule.isObjectBindingPattern(bound)) continue;
      const element = bound.elements.find(
        (el) => tsModule.isIdentifier(el.name) && el.name.text === name
      );
      if (!element) continue;

      // `{ schema }` reads `schema`; `{ schema: local }` reads `schema`.
      // A computed rename (`{ [k]: local }`) has no static source property,
      // so it yields nothing rather than a wrong guess.
      const source = element.propertyName
        ? propertyName(element.propertyName, tsModule)
        : name;
      if (source) return { fn, paramIndex: index, propertyName: source };
    }
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
  if (!callsCarrier(call, carrier.fn, checker, tsModule)) return undefined;

  const argument = call.arguments[carrier.paramIndex];
  if (!argument) return undefined;

  const schema =
    carrier.propertyName === undefined
      ? argument
      : propertyFromExpression(
          argument,
          carrier.propertyName,
          checker,
          tsModule
        );
  if (!schema) return undefined;

  const name =
    carrier.explicitName ??
    stringPropertyFromExpression(argument, 'name', checker, tsModule);
  return namedTarget(carrier.api, schema, sourceFile, tsModule, name);
}

function callsCarrier(
  call: ts.CallExpression,
  fn: ts.FunctionLikeDeclaration,
  checker: ts.TypeChecker,
  tsModule: typeof ts
): boolean {
  const resolved = checker.getResolvedSignature(call)?.declaration;
  if (resolved) {
    // Direct hit: the callee resolves to the wrapper itself. Signature
    // resolution already follows variables and factory return values, so
    // `wrap(...)` and `const w = wrap; w(...)` both land here.
    if (resolved === fn) return true;

    // Indirect hit: the wrapper is passed around under a function *type*
    // (`type Compile = (input: {schema: …}) => …`), so every call site resolves
    // to that type's call signature and the wrapper's own node is never seen.
    // Matching the annotation the wrapper was written against restores the link.
    if (contextualSignatureDeclarations(fn, checker, tsModule).has(resolved)) {
      return true;
    }
  }

  const symbol = checker.getSymbolAtLocation(call.expression);
  const aliased =
    symbol && (symbol.flags & tsModule.SymbolFlags.Alias)
      ? checker.getAliasedSymbol(symbol)
      : symbol;
  return aliased?.declarations?.some((decl) => decl === fn) ?? false;
}

/**
 * Call-signature declarations of the function type `fn` was written against —
 * its contextual type at the point it is defined (a return-type annotation, a
 * typed variable, a typed property).
 */
function contextualSignatureDeclarations(
  fn: ts.FunctionLikeDeclaration,
  checker: ts.TypeChecker,
  tsModule: typeof ts
): Set<ts.Node> {
  const declarations = new Set<ts.Node>();
  if (!tsModule.isArrowFunction(fn) && !tsModule.isFunctionExpression(fn)) {
    return declarations;
  }
  const contextual = checker.getContextualType(fn);
  if (!contextual) return declarations;
  for (const signature of contextual.getCallSignatures()) {
    if (signature.declaration) declarations.add(signature.declaration);
  }
  return declarations;
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

function namedTarget(
  api: string,
  expression: ts.Expression,
  sourceFile: ts.SourceFile,
  tsModule: typeof ts,
  explicitName?: string
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
  };
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
