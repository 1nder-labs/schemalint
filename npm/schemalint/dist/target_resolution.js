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
    if (!tsModule.isPropertyAccessExpression(expr))
        return undefined;
    if (!tsModule.isIdentifier(expr.expression))
        return undefined;
    const paramName = expr.expression.text;
    const fn = enclosingCarrierFunction(expr, paramName, tsModule);
    if (!fn)
        return undefined;
    return {
        api,
        fn,
        paramName,
        propertyName: expr.name.text,
        explicitName,
        metadata,
    };
}
function enclosingCarrierFunction(node, paramName, tsModule) {
    let current = node.parent;
    while (current) {
        if (tsModule.isFunctionDeclaration(current) ||
            tsModule.isFunctionExpression(current) ||
            tsModule.isArrowFunction(current) ||
            tsModule.isMethodDeclaration(current)) {
            const hasParam = current.parameters.some((param) => tsModule.isIdentifier(param.name) && param.name.text === paramName);
            if (hasParam)
                return current;
        }
        current = current.parent;
    }
    return undefined;
}
function carrierTargetFromCall(call, sourceFile, checker, tsModule, carrier) {
    if (!sameSymbol(call.expression, carrier.fn, checker, tsModule)) {
        return undefined;
    }
    const paramIndex = carrier.fn.parameters.findIndex((param) => tsModule.isIdentifier(param.name) && param.name.text === carrier.paramName);
    if (paramIndex === -1)
        return undefined;
    const schema = propertyFromExpression(call.arguments[paramIndex], carrier.propertyName, checker, tsModule);
    if (!schema)
        return undefined;
    const name = carrier.explicitName ??
        stringPropertyFromExpression(call.arguments[paramIndex], 'name', checker, tsModule);
    return namedTarget(carrier.api, schema, sourceFile, tsModule, name, {
        ...carrier.metadata,
        usageSpan: spanFor(call, sourceFile),
    });
}
function sameSymbol(expression, fn, checker, tsModule) {
    const symbol = checker.getSymbolAtLocation(expression);
    const aliased = symbol && (symbol.flags & tsModule.SymbolFlags.Alias)
        ? checker.getAliasedSymbol(symbol)
        : symbol;
    return aliased?.declarations?.some((decl) => decl === fn) ?? false;
}
function propertyFromObject(obj, name, checker, tsModule) {
    for (const prop of [...obj.properties].reverse()) {
        if (tsModule.isPropertyAssignment(prop)) {
            if (propertyName(prop.name, tsModule) === name)
                return prop.initializer;
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