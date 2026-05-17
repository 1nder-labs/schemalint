/**
 * Find top-level `const` declarations that are `z.object({...})` calls.
 */
export function findExportedSchemaCalls(sourceFile, tsModule) {
    const results = [];
    function walk(node) {
        if (tsModule.isVariableStatement(node) &&
            node.declarationList.declarations.length === 1) {
            const decl = node.declarationList.declarations[0];
            if (tsModule.isIdentifier(decl.name) &&
                decl.initializer) {
                const call = findZObjectCall(decl.initializer, tsModule);
                if (call) {
                    const entry = {
                        name: decl.name.text,
                        objectArg: call,
                    };
                    if (hasExportModifier(node, tsModule)) {
                        results.push(entry);
                    }
                }
            }
        }
        // Also handle `export default z.object({...})`
        if (tsModule.isExportAssignment(node) &&
            !node.isExportEquals &&
            tsModule.isCallExpression(node.expression)) {
            const call = findZObjectCall(node.expression, tsModule);
            if (call) {
                const entry = {
                    name: 'default',
                    objectArg: call,
                };
                results.push(entry);
            }
        }
        tsModule.forEachChild(node, walk);
    }
    tsModule.forEachChild(sourceFile, walk);
    return results;
}
/**
 * Scan a source file's import declarations for provider SDKs.
 * Returns "openai" or "anthropic" if detected, undefined otherwise.
 */
export function scanProviderImports(sourceFile, tsModule) {
    for (const stmt of sourceFile.statements) {
        if (!tsModule.isImportDeclaration(stmt))
            continue;
        const spec = stmt.moduleSpecifier;
        if (!tsModule.isStringLiteral(spec))
            continue;
        const mod = spec.text;
        if (mod === 'openai' || mod.startsWith('openai/')) {
            return 'openai';
        }
        if (mod === '@ai-sdk/openai' || mod.startsWith('@ai-sdk/openai/')) {
            return 'openai';
        }
        if (mod === '@anthropic-ai/sdk' || mod.startsWith('@anthropic-ai/')) {
            return 'anthropic';
        }
        if (mod === '@ai-sdk/anthropic' || mod.startsWith('@ai-sdk/anthropic/')) {
            return 'anthropic';
        }
    }
    return undefined;
}
export function hasExportModifier(node, tsModule) {
    if (!tsModule.canHaveModifiers(node))
        return false;
    const modifiers = tsModule.getModifiers(node);
    if (!modifiers)
        return false;
    for (const mod of modifiers) {
        if (mod.kind === tsModule.SyntaxKind.ExportKeyword)
            return true;
    }
    return false;
}
/**
 * Given a node, if it is `z.object({...})`, return the ObjectLiteralExpression argument.
 * Handles chaining: `z.object({...}).extend({...})` — returns the initial object.
 */
export function findZObjectCall(node, tsModule) {
    // Unwrap parenthesized expressions
    while (tsModule.isParenthesizedExpression(node)) {
        node = node.expression;
    }
    // Handle `export default z.object({...})` wrapped in another call expression
    // e.g., z.object({...}).extend({...})
    if (tsModule.isCallExpression(node) &&
        tsModule.isPropertyAccessExpression(node.expression)) {
        // Check if the inner node is a z.object() call
        const innerNode = node.expression.expression;
        if (tsModule.isCallExpression(innerNode) &&
            isZObjectCallExpression(innerNode, tsModule)) {
            const prop = innerNode.arguments[0];
            if (prop && tsModule.isObjectLiteralExpression(prop)) {
                return prop;
            }
        }
        // Check for .extend() / .merge() / .pick() / .omit() chaining on z.object()
        if (tsModule.isIdentifier(node.expression.name)) {
            const methodName = node.expression.name.text;
            if (methodName === 'extend' ||
                methodName === 'merge' ||
                methodName === 'pick' ||
                methodName === 'omit') {
                return findZObjectCall(node.expression.expression, tsModule);
            }
        }
    }
    // Direct z.object() call
    if (tsModule.isCallExpression(node) &&
        isZObjectCallExpression(node, tsModule)) {
        const arg = node.arguments[0];
        if (arg && tsModule.isObjectLiteralExpression(arg)) {
            return arg;
        }
    }
    return null;
}
/**
 * Check if a CallExpression is `z.object(...)`.
 */
function isZObjectCallExpression(node, tsModule) {
    const expr = node.expression;
    if (!tsModule.isPropertyAccessExpression(expr))
        return false;
    return (tsModule.isIdentifier(expr.expression) &&
        expr.expression.text === 'z' &&
        tsModule.isIdentifier(expr.name) &&
        expr.name.text === 'object');
}
/**
 * Walk an ObjectLiteralExpression (`{ email: z.string(), ... }`) and build a
 * source map mapping JSON Pointer paths to file:line locations.
 *
 * Handles nested `z.object({...})` by recursing into inner object literals.
 */
export function buildSourceMapFromObjectLiteral(objLit, sourceFile, tsModule) {
    const map = {};
    for (const prop of objLit.properties) {
        if (!tsModule.isPropertyAssignment(prop) ||
            !tsModule.isIdentifier(prop.name)) {
            continue;
        }
        const propName = prop.name.text;
        const { line } = sourceFile.getLineAndCharacterOfPosition(prop.getStart(sourceFile));
        const pointer = `/properties/${propName}`;
        map[pointer] = {
            file: sourceFile.fileName,
            line: line + 1, // 1-indexed
        };
        // Recurse into nested z.object() values
        const innerCall = findZObjectCall(prop.initializer, tsModule);
        if (innerCall) {
            const nested = buildSourceMapFromObjectLiteral(innerCall, sourceFile, tsModule);
            for (const [nestedPointer, nestedSpan] of Object.entries(nested)) {
                map[`/properties/${propName}${nestedPointer}`] = nestedSpan;
            }
        }
    }
    return map;
}
export function buildRootSourceMap(node, sourceFile) {
    const { line } = sourceFile.getLineAndCharacterOfPosition(node.getStart(sourceFile));
    return {
        '': {
            file: sourceFile.fileName,
            line: line + 1,
        },
    };
}
//# sourceMappingURL=discover_ast.js.map