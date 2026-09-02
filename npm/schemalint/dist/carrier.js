import { namedTarget, spanFor, } from './target_resolution.js';
import { propertyFromExpression, propertyName as propertyNameOf, stringPropertyFromExpression, } from './object_properties.js';
import { unwrapExpression } from './static_expression.js';
// ponytail: real wrapper chains don't nest this deep. A cycle between two
// carriers (or a pathological chain) stops here instead of hanging.
const CARRIER_HOP_LIMIT = 8;
export function pushExpressionOrCarrier(targets, carriers, api, expression, sourceFile, tsModule, explicitName, metadata) {
    const carrier = carrierExpression(api, expression, tsModule, explicitName, metadata);
    if (carrier) {
        carriers.push(carrier);
        return;
    }
    targets.push(namedTarget(api, expression, sourceFile, tsModule, explicitName, metadata));
}
/**
 * Resolves carriers to their call-site targets, following wrapper chains
 * multiple hops deep: a call site that forwards the schema through one of
 * its own parameters (bare, destructured, or re-spread) yields a new carrier
 * for its enclosing function rather than a dead end, and that carrier is fed
 * back in until no more forwarding is found or the hop limit is hit.
 */
export function collectCarrierTargets(program, fileSet, checker, tsModule, carriers) {
    const targets = [];
    const seen = new Set();
    let frontier = carriers;
    for (let hop = 0; frontier.length > 0 && hop < CARRIER_HOP_LIMIT; hop++) {
        const active = frontier.filter((carrier) => {
            const key = carrierKey(carrier);
            if (seen.has(key))
                return false;
            seen.add(key);
            return true;
        });
        if (active.length === 0)
            break;
        const nextFrontier = [];
        const carrierSignatures = active.map((carrier) => ({
            carrier,
            signatures: contextualSignatureDeclarations(carrier.fn, checker, tsModule),
        }));
        for (const sourceFile of program.getSourceFiles()) {
            if (sourceFile.isDeclarationFile ||
                sourceFile.fileName.includes('node_modules') ||
                !fileSet.has(sourceFile.fileName)) {
                continue;
            }
            function walk(node) {
                if (tsModule.isCallExpression(node)) {
                    for (const { carrier, signatures } of carrierSignatures) {
                        const result = carrierTargetFromCall(node, sourceFile, checker, tsModule, carrier, signatures);
                        if (!result)
                            continue;
                        if (result.kind === 'target') {
                            targets.push(result.target);
                        }
                        else {
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
function carrierKey(carrier) {
    return `${carrier.fn.getSourceFile().fileName}:${carrier.fn.pos}:${carrier.paramIndex}:${carrier.propertyName ?? ''}`;
}
function carrierExpression(api, expression, tsModule, explicitName, metadata) {
    const expr = unwrapExpression(expression, tsModule);
    // `opts.schema` — the schema is a property of a whole parameter.
    if (tsModule.isPropertyAccessExpression(expr) &&
        tsModule.isIdentifier(expr.expression)) {
        const param = carrierParam(expr, expr.expression.text, tsModule);
        // A destructured binding is already a property read; `opts.schema` on top
        // of one would be a second hop this doesn't follow.
        if (!param || param.propertyName !== undefined)
            return undefined;
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
function carrierParam(node, name, tsModule) {
    for (let current = node.parent; current; current = current.parent) {
        if (!tsModule.isFunctionDeclaration(current) &&
            !tsModule.isFunctionExpression(current) &&
            !tsModule.isArrowFunction(current) &&
            !tsModule.isMethodDeclaration(current)) {
            continue;
        }
        const fn = current;
        for (let index = 0; index < fn.parameters.length; index++) {
            const bound = fn.parameters[index].name;
            if (tsModule.isIdentifier(bound)) {
                if (bound.text === name)
                    return { fn, paramIndex: index };
                continue;
            }
            if (!tsModule.isObjectBindingPattern(bound))
                continue;
            const element = bound.elements.find((el) => tsModule.isIdentifier(el.name) && el.name.text === name);
            if (!element)
                continue;
            // `{ schema }` reads `schema`; `{ schema: local }` reads `schema`.
            // A computed rename (`{ [k]: local }`) has no static source property,
            // so it yields nothing rather than a wrong guess.
            const source = element.propertyName
                ? propertyNameOf(element.propertyName, tsModule)
                : name;
            if (source)
                return { fn, paramIndex: index, propertyName: source };
        }
    }
    return undefined;
}
function carrierTargetFromCall(call, sourceFile, checker, tsModule, carrier, contextualSignatures) {
    if (!callsCarrier(call, carrier.fn, contextualSignatures, checker, tsModule)) {
        return undefined;
    }
    const argument = call.arguments[carrier.paramIndex];
    if (!argument)
        return undefined;
    const step = resolveCarrierArgument(argument, carrier, checker, tsModule);
    if (!step)
        return undefined;
    if (step.kind === 'carrier') {
        return { kind: 'carrier', carrier: step.carrier };
    }
    const name = carrier.explicitName ??
        stringPropertyFromExpression(argument, 'name', checker, tsModule);
    return {
        kind: 'target',
        target: namedTarget(carrier.api, step.expression, sourceFile, tsModule, name, {
            ...carrier.metadata,
            usageSpan: spanFor(call, sourceFile),
        }),
    };
}
/**
 * Recovers the schema at one hop of a carrier chain: either a static
 * expression the target can be built from, or — when the argument merely
 * forwards it through the *enclosing* function's own parameter (bare, or
 * re-wrapped in an object literal via `{ ...params }` / `{ schema:
 * params.schema }`) — a new carrier for that enclosing function to resolve
 * at its own call sites.
 */
function resolveCarrierArgument(argument, carrier, checker, tsModule) {
    const propertyName = carrier.propertyName;
    const candidate = propertyName === undefined
        ? argument
        : propertyFromExpression(argument, propertyName, checker, tsModule);
    if (candidate) {
        // The value this hop recovered might itself just be forwarding a
        // parameter one level further out (`f(params.schema)` where `params` is
        // the caller's own parameter) — reuse the same top-level matcher rather
        // than treating it as final.
        const derived = carrierExpression(carrier.api, candidate, tsModule, carrier.explicitName, carrier.metadata);
        return derived
            ? { kind: 'carrier', carrier: derived }
            : { kind: 'expression', expression: candidate };
    }
    // Static drilling failed — the value may be forwarded via the enclosing
    // function's own parameter that this hop can't see through directly: a
    // bare passthrough (`f(params, ...)`) or an object literal that re-spreads
    // it (`f({ ...params }, ...)`).
    if (propertyName === undefined)
        return undefined;
    const base = spreadSource(argument, tsModule) ?? argument;
    if (!tsModule.isIdentifier(base))
        return undefined;
    const param = carrierParam(base, base.text, tsModule);
    if (!param)
        return undefined;
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
function spreadSource(expr, tsModule) {
    if (!tsModule.isObjectLiteralExpression(expr))
        return undefined;
    for (const prop of expr.properties) {
        if (tsModule.isSpreadAssignment(prop))
            return prop.expression;
    }
    return undefined;
}
function callsCarrier(call, fn, contextualSignatures, checker, tsModule) {
    const resolved = checker.getResolvedSignature(call)?.declaration;
    if (resolved) {
        // Direct hit: the callee resolves to the wrapper itself. Signature
        // resolution already follows variables and factory return values.
        if (resolved === fn)
            return true;
        // Indirect hit: the wrapper is called under a function *type* it was
        // written against (`type Compile = ...`), so every call site resolves to
        // that type's signature and the wrapper's own node is never seen.
        if (contextualSignatures.has(resolved)) {
            return true;
        }
    }
    const symbol = checker.getSymbolAtLocation(call.expression);
    const aliased = symbol && (symbol.flags & tsModule.SymbolFlags.Alias)
        ? checker.getAliasedSymbol(symbol)
        : symbol;
    return aliased?.declarations?.some((decl) => decl === fn) ?? false;
}
/**
 * Call-signature declarations of the function type `fn` was written against —
 * its contextual type at the point it is defined (a return-type annotation, a
 * typed variable, a typed property).
 */
function contextualSignatureDeclarations(fn, checker, tsModule) {
    const declarations = new Set();
    if (!tsModule.isArrowFunction(fn) && !tsModule.isFunctionExpression(fn)) {
        return declarations;
    }
    const contextual = checker.getContextualType(fn);
    if (!contextual)
        return declarations;
    for (const signature of contextual.getCallSignatures()) {
        if (signature.declaration)
            declarations.add(signature.declaration);
    }
    return declarations;
}
//# sourceMappingURL=carrier.js.map