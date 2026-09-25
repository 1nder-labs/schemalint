import type * as ts from 'typescript';

import {
  namedTarget,
  spanFor,
  type TargetExpression,
  type TargetMetadata,
} from './target_resolution.js';
import {
  propertyFromExpression,
  propertyName as propertyNameOf,
  stringPropertyFromExpression,
} from './object_properties.js';
import { unwrapExpression } from './static_expression.js';
import {
  calleeName,
  callsCarrier,
  carrierBaseNames,
  collectInvocationAliases,
  contextualSignatureDeclarations,
  expandCarrierNames,
} from './carrier_calls.js';

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

// ponytail: real wrapper chains don't nest this deep. A cycle between two
// carriers (or a pathological chain) stops here instead of hanging.
const CARRIER_HOP_LIMIT = 8;

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

/**
 * Resolves carriers to their call-site targets, following wrapper chains
 * multiple hops deep: a call site that forwards the schema through one of
 * its own parameters (bare, destructured, or re-spread) yields a new carrier
 * for its enclosing function rather than a dead end, and that carrier is fed
 * back in until no more forwarding is found or the hop limit is hit.
 */
export function collectCarrierTargets(
  program: ts.Program,
  fileSet: ReadonlySet<string>,
  checker: ts.TypeChecker,
  tsModule: typeof ts,
  carriers: CarrierExpression[]
): TargetExpression[] {
  const targets: TargetExpression[] = [];
  const seen = new Set<string>();
  let frontier = carriers;

  for (let hop = 0; frontier.length > 0 && hop < CARRIER_HOP_LIMIT; hop++) {
    const active = frontier.filter((carrier) => {
      const key = carrierKey(carrier);
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    });
    if (active.length === 0) break;

    const nextFrontier: CarrierExpression[] = [];
    const carrierSignatures = active.map((carrier) => ({
      carrier,
      signatures: contextualSignatureDeclarations(carrier.fn, checker, tsModule),
    }));

    // Syntactic pre-filter: `callsCarrier` resolves a signature per call per
    // carrier, so every unremarkable call in every selected file pays for
    // signature instantiation. Each carrier instead gets a superset of the
    // names it can be invoked under (undefined when no such superset exists
    // syntactically), and only calls matching arity plus a plausible name
    // reach the checker.
    const baseNames = carrierSignatures.map(({ carrier, signatures }) =>
      carrierBaseNames(carrier.fn, signatures, tsModule)
    );
    const aliases = baseNames.some((names) => names !== undefined)
      ? collectInvocationAliases(program, fileSet, tsModule)
      : undefined;
    const carrierNames = baseNames.map((names) =>
      names && aliases ? expandCarrierNames(names, aliases) : undefined
    );

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
          for (let index = 0; index < carrierSignatures.length; index++) {
            const { carrier, signatures } = carrierSignatures[index];
            const result = carrierTargetFromCall(
              node,
              sourceFile,
              checker,
              tsModule,
              carrier,
              signatures,
              carrierNames[index]
            );
            if (!result) continue;
            if (result.kind === 'target') {
              targets.push(result.target);
            } else {
              nextFrontier.push(result.carrier);
            }
          }
        }
        tsModule.forEachChild(node, walk);
      }

      tsModule.forEachChild(sourceFile, walk);
    }

    frontier = nextFrontier;
  }

  return targets;
}

function carrierKey(carrier: CarrierExpression): string {
  return `${carrier.fn.getSourceFile().fileName}:${carrier.fn.pos}:${carrier.paramIndex}:${carrier.propertyName ?? ''}`;
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
        ? propertyNameOf(element.propertyName, tsModule)
        : name;
      if (source) return { fn, paramIndex: index, propertyName: source };
    }
  }
  return undefined;
}

type CarrierHopResult =
  | { kind: 'target'; target: TargetExpression }
  | { kind: 'carrier'; carrier: CarrierExpression };

function carrierTargetFromCall(
  call: ts.CallExpression,
  sourceFile: ts.SourceFile,
  checker: ts.TypeChecker,
  tsModule: typeof ts,
  carrier: CarrierExpression,
  contextualSignatures: ReadonlySet<ts.Node>,
  names: ReadonlySet<string> | undefined
): CarrierHopResult | undefined {
  // A missing argument is a guaranteed non-match — check before touching
  // the checker.
  if (call.arguments.length <= carrier.paramIndex) return undefined;

  // Name filter: `names` undefined means the carrier is invocable under
  // anything (factory return, dynamic export, typed-alias injection); an
  // undeterminable callee shape is likewise never filtered out.
  if (names !== undefined) {
    const name = calleeName(call, tsModule);
    if (name !== undefined && !names.has(name)) return undefined;
  }

  if (!callsCarrier(call, carrier.fn, contextualSignatures, checker, tsModule)) {
    return undefined;
  }

  const argument = call.arguments[carrier.paramIndex];
  if (!argument) return undefined;

  const step = resolveCarrierArgument(argument, carrier, checker, tsModule);
  if (!step) return undefined;

  if (step.kind === 'carrier') {
    return { kind: 'carrier', carrier: step.carrier };
  }

  const name =
    carrier.explicitName ??
    stringPropertyFromExpression(argument, 'name', checker, tsModule);
  return {
    kind: 'target',
    target: namedTarget(carrier.api, step.expression, sourceFile, tsModule, name, {
      ...carrier.metadata,
      usageSpan: spanFor(call, sourceFile),
    }),
  };
}

type CarrierStep =
  | { kind: 'expression'; expression: ts.Expression }
  | { kind: 'carrier'; carrier: CarrierExpression };

/**
 * Recovers the schema at one hop of a carrier chain: either a static
 * expression the target can be built from, or — when the argument merely
 * forwards it through the *enclosing* function's own parameter (bare, or
 * re-wrapped in an object literal via `{ ...params }` / `{ schema:
 * params.schema }`) — a new carrier for that enclosing function to resolve
 * at its own call sites.
 */
function resolveCarrierArgument(
  argument: ts.Expression,
  carrier: CarrierExpression,
  checker: ts.TypeChecker,
  tsModule: typeof ts
): CarrierStep | undefined {
  const propertyName = carrier.propertyName;
  const candidate =
    propertyName === undefined
      ? argument
      : propertyFromExpression(argument, propertyName, checker, tsModule);

  if (candidate) {
    // The value this hop recovered might itself just be forwarding a
    // parameter one level further out (`f(params.schema)` where `params` is
    // the caller's own parameter) — reuse the same top-level matcher rather
    // than treating it as final.
    const derived = carrierExpression(
      carrier.api,
      candidate,
      tsModule,
      carrier.explicitName,
      carrier.metadata
    );
    return derived
      ? { kind: 'carrier', carrier: derived }
      : { kind: 'expression', expression: candidate };
  }

  // Static drilling failed — the value may be forwarded via the enclosing
  // function's own parameter that this hop can't see through directly: a
  // bare passthrough (`f(params, ...)`) or an object literal that re-spreads
  // it (`f({ ...params }, ...)`).
  if (propertyName === undefined) return undefined;

  const base = spreadSource(argument, tsModule) ?? argument;
  if (!tsModule.isIdentifier(base)) return undefined;

  const param = carrierParam(base, base.text, tsModule);
  if (!param) return undefined;

  return {
    kind: 'carrier',
    carrier: {
      ...param,
      api: carrier.api,
      propertyName: param.propertyName ?? propertyName,
      explicitName: carrier.explicitName,
      metadata: carrier.metadata,
    },
  };
}

/** The spread source of an object literal's *first* spread property, if any. */
function spreadSource(
  expr: ts.Expression,
  tsModule: typeof ts
): ts.Expression | undefined {
  if (!tsModule.isObjectLiteralExpression(expr)) return undefined;
  for (const prop of expr.properties) {
    if (tsModule.isSpreadAssignment(prop)) return prop.expression;
  }
  return undefined;
}


