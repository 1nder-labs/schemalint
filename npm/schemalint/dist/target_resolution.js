import { distinctExpressions, staticAlternatives, unambiguousExpression, unwrapExpression, } from './static_expression.js';
export function pushExpressionOrCarrier(targets, carriers, api, expression, sourceFile, tsModule, explicitName, metadata) {
    const carrier = carrierExpression(api, expression, tsModule, explicitName, metadata);
    if (carrier) {
        carriers.push(carrier);
        return;
    }
    targets.push(namedTarget(api, expression, sourceFile, tsModule, explicitName, metadata));
}
export function collectCarrierTargets(program, fileSet, checker, tsModule, carriers) {
    if (carriers.length === 0)
        return [];
    const targets = [];
    for (const sourceFile of program.getSourceFiles()) {
        if (sourceFile.isDeclarationFile ||
            sourceFile.fileName.includes('node_modules') ||
            !fileSet.has(sourceFile.fileName)) {
            continue;
        }
        function walk(node) {
            if (tsModule.isCallExpression(node)) {
                for (const carrier of carriers) {
                    const target = carrierTargetFromCall(node, sourceFile, checker, tsModule, carrier);
                    if (target)
                        targets.push(target);
                }
            }
            tsModule.forEachChild(node, walk);
        }
        tsModule.forEachChild(sourceFile, walk);
    }
    return targets;
}
export function propertyFromExpression(expr, name, checker, tsModule) {
    const candidates = [];
    const containers = staticAlternatives(expr, checker, tsModule);
    if (containers.length === 0)
        return undefined;
    for (const container of containers) {
        if (!tsModule.isObjectLiteralExpression(container))
            return undefined;
        const property = propertyFromObject(container, name, checker, tsModule);
        const stable = unambiguousExpression(property, checker, tsModule);
        if (!stable)
            return undefined;
        candidates.push(stable);
    }
    const distinct = distinctExpressions(candidates, checker, tsModule);
    return distinct.length === 1 ? distinct[0] : undefined;
}
export function stringPropertyFromExpression(expr, name, checker, tsModule) {
    const value = propertyFromExpression(expr, name, checker, tsModule);
    return stringLiteralText(value, tsModule);
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
                ? propertyName(element.propertyName, tsModule)
                : name;
            if (source)
                return { fn, paramIndex: index, propertyName: source };
        }
    }
    return undefined;
}
function carrierTargetFromCall(call, sourceFile, checker, tsModule, carrier) {
    if (!callsCarrier(call, carrier.fn, checker, tsModule))
        return undefined;
    const argument = call.arguments[carrier.paramIndex];
    if (!argument)
        return undefined;
    const schema = carrier.propertyName === undefined
        ? argument
        : propertyFromExpression(argument, carrier.propertyName, checker, tsModule);
    if (!schema)
        return undefined;
    const name = carrier.explicitName ??
        stringPropertyFromExpression(argument, 'name', checker, tsModule);
    return namedTarget(carrier.api, schema, sourceFile, tsModule, name, {
        ...carrier.metadata,
        usageSpan: spanFor(call, sourceFile),
    });
}
function callsCarrier(call, fn, checker, tsModule) {
    const resolved = checker.getResolvedSignature(call)?.declaration;
    if (resolved) {
        // Direct hit: the callee resolves to the wrapper itself. Signature
        // resolution already follows variables and factory return values, so
        // `wrap(...)` and `const w = wrap; w(...)` both land here.
        if (resolved === fn)
            return true;
        // Indirect hit: the wrapper is passed around under a function *type*
        // (`type Compile = (input: {schema: …}) => …`), so every call site resolves
        // to that type's call signature and the wrapper's own node is never seen.
        // Matching the annotation the wrapper was written against restores the link.
        if (contextualSignatureDeclarations(fn, checker, tsModule).has(resolved)) {
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
function propertyFromObject(obj, name, checker, tsModule) {
    for (const prop of [...obj.properties].reverse()) {
        if (tsModule.isPropertyAssignment(prop)) {
            if (propertyName(prop.name, tsModule) === name)
                return prop.initializer;
            continue;
        }
        // `{ schema }` — the value is the name itself.
        if (tsModule.isShorthandPropertyAssignment(prop)) {
            if (prop.name.text === name)
                return prop.name;
            continue;
        }
        if (tsModule.isSpreadAssignment(prop)) {
            const fromSpread = propertyFromExpression(prop.expression, name, checker, tsModule);
            if (fromSpread)
                return fromSpread;
        }
    }
    return undefined;
}
function namedTarget(api, expression, sourceFile, tsModule, explicitName, metadata) {
    const { line } = sourceFile.getLineAndCharacterOfPosition(expression.getStart(sourceFile));
    const expr = unwrapExpression(expression, tsModule);
    const suffix = explicitName ??
        (tsModule.isIdentifier(expr) ? expr.text : `inline:${line + 1}`);
    return {
        name: `${api}:${suffix}`,
        sourceFile,
        expression,
        metadata,
    };
}
export function spanFor(node, sourceFile) {
    const { line, character } = sourceFile.getLineAndCharacterOfPosition(node.getStart(sourceFile));
    return { file: sourceFile.fileName, line: line + 1, col: character + 1 };
}
export function stringValueFromExpression(expr, checker, tsModule) {
    const values = staticAlternatives(expr, checker, tsModule).map((alternative) => stringLiteralText(alternative, tsModule));
    if (values.length === 0 || values.some((value) => value === undefined)) {
        return undefined;
    }
    const distinct = new Set(values);
    return distinct.size === 1 ? values[0] : undefined;
}
function stringLiteralText(expr, tsModule) {
    return expr && tsModule.isStringLiteralLike(expr) ? expr.text : undefined;
}
function propertyName(name, tsModule) {
    if (tsModule.isIdentifier(name) || tsModule.isStringLiteral(name)) {
        return name.text;
    }
    return undefined;
}
//# sourceMappingURL=target_resolution.js.map