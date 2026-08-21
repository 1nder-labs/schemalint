import type * as ts from 'typescript';

import type {
  EnvelopeField,
  ProviderResolution,
  TargetSpan,
} from './sdk_adapters.js';
import {
  propertyFromExpression,
  propertyName,
  stringLiteralText,
  stringPropertyFromExpression,
} from './object_properties.js';
import {
  distinctExpressions,
  staticAlternatives,
  unambiguousExpression,
  unwrapExpression,
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
  paramIndex: number;
  /**
   * Property to read off the call argument. Undefined when the parameter
   * *is* the schema (`f(schema)`), in which case the argument is used whole.
   */
  propertyName?: string;
  explicitName?: string;
  metadata: TargetMetadata;
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
  const carrierSignatures = carriers.map((carrier) => ({
    carrier,
    signatures: contextualSignatureDeclarations(carrier.fn, checker, tsModule),
  }));
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
        for (const { carrier, signatures } of carrierSignatures) {
          const target = carrierTargetFromCall(
            node,
            sourceFile,
            checker,
            tsModule,
            carrier,
            signatures
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



function carrierExpression(
  api: string,
  expression: ts.Expression,
  tsModule: typeof ts,
  explicitName: string | undefined,
  metadata: TargetMetadata
): CarrierExpression | undefined {
  const expr = unwrapExpression(expression, tsModule);

  // `opts.schema` — the schema is a property of a whole parameter.
  if (
    tsModule.isPropertyAccessExpression(expr) &&
    tsModule.isIdentifier(expr.expression)
  ) {
    const param = carrierParam(expr, expr.expression.text, tsModule);
    // A destructured binding is already a property read; `opts.schema` on top
    // of one would be a second hop this doesn't follow.
    if (!param || param.propertyName !== undefined) return undefined;
    return {
      ...param,
      api,
      propertyName: expr.name.text,
      explicitName,
      metadata,
    };
  }

  // A bare `schema` — either the parameter itself (`f(schema)`) or destructured
  // out of a parameter object (`f({ schema })`). Both arrive at the call site
  // inside the same argument; `carrierParam` reports which read recovers it.
  if (tsModule.isIdentifier(expr)) {
    const param = carrierParam(expr, expr.text, tsModule);
    return param && { ...param, api, explicitName, metadata };
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
  carrier: CarrierExpression,
  contextualSignatures: ReadonlySet<ts.Node>
): TargetExpression | undefined {
  if (!callsCarrier(call, carrier.fn, contextualSignatures, checker, tsModule)) {
    return undefined;
  }

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
  return namedTarget(carrier.api, schema, sourceFile, tsModule, name, {
    ...carrier.metadata,
    usageSpan: spanFor(call, sourceFile),
  });
}

function callsCarrier(
  call: ts.CallExpression,
  fn: ts.FunctionLikeDeclaration,
  contextualSignatures: ReadonlySet<ts.Node>,
  checker: ts.TypeChecker,
  tsModule: typeof ts
): boolean {
  const resolved = checker.getResolvedSignature(call)?.declaration;
  if (resolved) {
    // Direct hit: the callee resolves to the wrapper itself. Signature
    // resolution already follows variables and factory return values.
    if (resolved === fn) return true;
    // Indirect hit: the wrapper is called under a function *type* it was
    // written against (`type Compile = ...`), so every call site resolves to
    // that type's signature and the wrapper's own node is never seen.
    if (contextualSignatures.has(resolved)) {
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
  const expr = unwrapExpression(expression, tsModule);
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
  const values = staticAlternatives(expr, checker, tsModule).map((alternative) =>
    stringLiteralText(alternative, tsModule)
  );
  if (values.length === 0 || values.some((value) => value === undefined)) {
    return undefined;
  }
  const distinct = new Set(values);
  return distinct.size === 1 ? values[0] : undefined;
}
