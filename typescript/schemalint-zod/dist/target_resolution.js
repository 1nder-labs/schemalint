export function pushExpressionOrCarrier(targets, carriers, api, expression, sourceFile, tsModule, explicitName) {
    const carrier = carrierExpression(api, expression, tsModule, explicitName);
    if (carrier) {
        carriers.push(carrier);
        return;
    }
    targets.push(namedTarget(api, expression, sourceFile, tsModule, explicitName));
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
export function objectExpression(expr, checker, tsModule) {
    if (!expr)
        return undefined;
    const unwrapped = skipParens(expr, tsModule);
    if (tsModule.isObjectLiteralExpression(unwrapped))
        return unwrapped;
    if (tsModule.isIdentifier(unwrapped)) {
        const decl = resolveVariableDeclaration(unwrapped, checker, tsModule);
        if (decl?.initializer) {
            return objectExpression(decl.initializer, checker, tsModule);
        }
    }
    if (tsModule.isConditionalExpression(unwrapped)) {
        return (objectExpression(unwrapped.whenTrue, checker, tsModule) ??
            objectExpression(unwrapped.whenFalse, checker, tsModule));
    }
    return undefined;
}
export function propertyFromExpression(expr, name, checker, tsModule) {
    if (!expr)
        return undefined;
    const unwrapped = skipParens(expr, tsModule);
    if (tsModule.isObjectLiteralExpression(unwrapped)) {
        return propertyFromObject(unwrapped, name, checker, tsModule);
    }
    if (tsModule.isIdentifier(unwrapped)) {
        const decl = resolveVariableDeclaration(unwrapped, checker, tsModule);
        return propertyFromExpression(decl?.initializer, name, checker, tsModule);
    }
    if (tsModule.isConditionalExpression(unwrapped)) {
        return (propertyFromExpression(unwrapped.whenTrue, name, checker, tsModule) ??
            propertyFromExpression(unwrapped.whenFalse, name, checker, tsModule));
    }
    return undefined;
}
export function stringPropertyFromExpression(expr, name, checker, tsModule) {
    const value = propertyFromExpression(expr, name, checker, tsModule);
    return stringLiteralText(value, tsModule);
}
export function resolveVariableDeclaration(id, checker, tsModule) {
    const symbol = checker.getSymbolAtLocation(id);
    const aliased = symbol && (symbol.flags & tsModule.SymbolFlags.Alias)
        ? checker.getAliasedSymbol(symbol)
        : symbol;
    const decl = aliased?.valueDeclaration ?? aliased?.declarations?.[0];
    return decl && tsModule.isVariableDeclaration(decl) ? decl : undefined;
}
function carrierExpression(api, expression, tsModule, explicitName) {
    const expr = skipParens(expression, tsModule);
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
    return namedTarget(carrier.api, schema, sourceFile, tsModule, name);
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
function namedTarget(api, expression, sourceFile, tsModule, explicitName) {
    const { line } = sourceFile.getLineAndCharacterOfPosition(expression.getStart(sourceFile));
    const expr = skipParens(expression, tsModule);
    const suffix = explicitName ??
        (tsModule.isIdentifier(expr) ? expr.text : `inline:${line + 1}`);
    return {
        name: `${api}:${suffix}`,
        sourceFile,
        expression,
    };
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
function skipParens(node, tsModule) {
    while (tsModule.isParenthesizedExpression(node))
        node = node.expression;
    return node;
}
//# sourceMappingURL=target_resolution.js.map