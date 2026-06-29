import { resolveTarget } from './target_emit.js';
import { collectTargetImports, FORMAT_CALLS, isProviderCall, OUTPUT_CALLS, TOOL_CALLS, } from './target_imports.js';
import { collectCarrierTargets, objectExpression, propertyFromExpression, pushExpressionOrCarrier, } from './target_resolution.js';
export function findSchemaTargets(program, fileSet, tsModule, compilerOptions) {
    const checker = program.getTypeChecker();
    const carrierExpressions = [];
    const targets = [];
    const seen = new Set();
    for (const sourceFile of program.getSourceFiles()) {
        if (sourceFile.isDeclarationFile ||
            sourceFile.fileName.includes('node_modules')) {
            continue;
        }
        if (!fileSet.has(sourceFile.fileName))
            continue;
        const found = collectTargetExpressions(sourceFile, checker, tsModule, carrierExpressions);
        for (const target of found) {
            pushTarget(targets, seen, resolveTarget(target, checker, tsModule, compilerOptions));
        }
    }
    for (const target of collectCarrierTargets(program, fileSet, checker, tsModule, carrierExpressions)) {
        pushTarget(targets, seen, resolveTarget(target, checker, tsModule, compilerOptions));
    }
    return targets;
}
function collectTargetExpressions(sourceFile, checker, tsModule, carrierExpressions) {
    const targets = [];
    const imports = collectTargetImports(sourceFile, tsModule);
    function walk(node) {
        if (tsModule.isCallExpression(node)) {
            collectFromCall(node, sourceFile, checker, tsModule, imports, targets, carrierExpressions);
        }
        tsModule.forEachChild(node, walk);
    }
    tsModule.forEachChild(sourceFile, walk);
    return targets;
}
function collectFromCall(call, sourceFile, checker, tsModule, imports, targets, carrierExpressions) {
    const ref = callRef(call.expression, tsModule);
    if (!ref || !isProviderCall(ref, imports))
        return;
    const name = ref.name;
    if (OUTPUT_CALLS.has(name)) {
        const schema = propertyFromExpression(call.arguments[0], 'schema', checker, tsModule);
        if (schema) {
            pushExpressionOrCarrier(targets, carrierExpressions, name, schema, sourceFile, tsModule);
        }
        return;
    }
    if (name === 'object' && isOutputObjectCall(call, tsModule, imports)) {
        const schema = propertyFromExpression(call.arguments[0], 'schema', checker, tsModule);
        if (schema) {
            pushExpressionOrCarrier(targets, carrierExpressions, 'Output.object', schema, sourceFile, tsModule);
        }
        return;
    }
    if (TOOL_CALLS.has(name)) {
        const arg = objectExpression(call.arguments[0], checker, tsModule);
        const schema = arg &&
            (propertyExpression(arg, 'inputSchema', tsModule) ??
                propertyExpression(arg, 'parameters', tsModule));
        if (schema) {
            pushExpressionOrCarrier(targets, carrierExpressions, name, schema, sourceFile, tsModule, stringProperty(arg, 'name', tsModule));
        }
        return;
    }
    if (FORMAT_CALLS.has(name) && call.arguments[0]) {
        const schemaName = stringLiteralText(call.arguments[1], tsModule);
        pushExpressionOrCarrier(targets, carrierExpressions, name, call.arguments[0], sourceFile, tsModule, schemaName);
    }
}
function pushTarget(targets, seen, resolved) {
    const key = `${resolved.filePath}:${resolved.exportName}:${resolved.name}`;
    if (seen.has(key))
        return;
    seen.add(key);
    targets.push(resolved);
}
function propertyExpression(obj, name, tsModule) {
    for (const prop of obj.properties) {
        if (!tsModule.isPropertyAssignment(prop))
            continue;
        if (propertyName(prop.name, tsModule) === name)
            return prop.initializer;
    }
    return undefined;
}
function stringProperty(obj, name, tsModule) {
    const expr = propertyExpression(obj, name, tsModule);
    return stringLiteralText(expr, tsModule);
}
function stringLiteralText(expr, tsModule) {
    return expr && tsModule.isStringLiteralLike(expr) ? expr.text : undefined;
}
function propertyName(name, tsModule) {
    if (tsModule.isIdentifier(name) || tsModule.isStringLiteral(name))
        return name.text;
    return undefined;
}
function callRef(expr, tsModule) {
    if (tsModule.isIdentifier(expr))
        return { name: expr.text };
    if (tsModule.isPropertyAccessExpression(expr) &&
        tsModule.isIdentifier(expr.expression)) {
        return { name: expr.name.text, receiver: expr.expression.text };
    }
    return undefined;
}
function isOutputObjectCall(call, tsModule, imports) {
    const expr = call.expression;
    return tsModule.isPropertyAccessExpression(expr) &&
        tsModule.isIdentifier(expr.expression) &&
        imports.outputObjects.has(expr.expression.text) &&
        expr.name.text === 'object';
}
//# sourceMappingURL=targets.js.map