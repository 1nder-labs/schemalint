import { pathToFileURL } from 'node:url';
import { buildRootSourceMap, buildSourceMapFromObjectLiteral, findZObjectCall, hasExportModifier, } from './discover_ast.js';
import { collectTargetImports, FORMAT_CALLS, isProviderCall, OUTPUT_CALLS, TOOL_CALLS, } from './target_imports.js';
export function findSchemaTargets(program, fileSet, tsModule, compilerOptions) {
    const checker = program.getTypeChecker();
    const targets = [];
    const seen = new Set();
    for (const sourceFile of program.getSourceFiles()) {
        if (sourceFile.isDeclarationFile ||
            sourceFile.fileName.includes('node_modules')) {
            continue;
        }
        if (!fileSet.has(sourceFile.fileName))
            continue;
        for (const target of collectTargetExpressions(sourceFile, tsModule)) {
            const resolved = resolveTarget(target, sourceFile, checker, tsModule, compilerOptions);
            const key = `${resolved.filePath}:${resolved.exportName}:${resolved.name}`;
            if (!seen.has(key)) {
                seen.add(key);
                targets.push(resolved);
            }
        }
    }
    return targets;
}
function collectTargetExpressions(sourceFile, tsModule) {
    const targets = [];
    const imports = collectTargetImports(sourceFile, tsModule);
    function walk(node) {
        if (tsModule.isCallExpression(node)) {
            collectFromCall(node, sourceFile, tsModule, imports, targets);
        }
        tsModule.forEachChild(node, walk);
    }
    tsModule.forEachChild(sourceFile, walk);
    return targets;
}
function collectFromCall(call, sourceFile, tsModule, imports, targets) {
    const ref = callRef(call.expression, tsModule);
    if (!ref || !isProviderCall(ref, imports))
        return;
    const name = ref.name;
    if (OUTPUT_CALLS.has(name)) {
        const arg = firstObjectArg(call, tsModule);
        const schema = arg ? propertyExpression(arg, 'schema', tsModule) : undefined;
        if (schema)
            targets.push(namedTarget(name, schema, sourceFile, tsModule));
        return;
    }
    if (name === 'object' && isOutputObjectCall(call, tsModule, imports)) {
        const arg = firstObjectArg(call, tsModule);
        const schema = arg ? propertyExpression(arg, 'schema', tsModule) : undefined;
        if (schema) {
            targets.push(namedTarget('Output.object', schema, sourceFile, tsModule));
        }
        return;
    }
    if (TOOL_CALLS.has(name)) {
        const arg = firstObjectArg(call, tsModule);
        const schema = arg &&
            (propertyExpression(arg, 'inputSchema', tsModule) ??
                propertyExpression(arg, 'parameters', tsModule));
        if (schema) {
            targets.push(namedTarget(name, schema, sourceFile, tsModule, stringProperty(arg, 'name', tsModule)));
        }
        return;
    }
    if (FORMAT_CALLS.has(name) && call.arguments[0]) {
        const schemaName = stringLiteralText(call.arguments[1], tsModule);
        targets.push(namedTarget(name, call.arguments[0], sourceFile, tsModule, schemaName));
    }
}
function resolveTarget(target, sourceFile, checker, tsModule, compilerOptions) {
    const expr = skipParens(target.expression, tsModule);
    const sourceMap = sourceMapForTarget(expr, sourceFile, checker, tsModule);
    if (tsModule.isIdentifier(expr)) {
        const exported = resolveExportedIdentifier(expr, checker, tsModule);
        if (exported) {
            return {
                name: target.name,
                filePath: exported.filePath,
                exportName: exported.exportName,
                sourceMap,
            };
        }
    }
    const exportName = `__schemalint_target_${safeName(target.name)}`;
    return {
        name: target.name,
        filePath: sourceFile.fileName,
        exportName,
        sourceMap,
        syntheticSource: buildSyntheticModule(sourceFile, expr, exportName, tsModule, compilerOptions),
    };
}
function resolveExportedIdentifier(id, checker, tsModule) {
    const decl = resolveVariableDeclaration(id, checker, tsModule);
    if (!decl)
        return undefined;
    if (tsModule.isVariableDeclaration(decl) && tsModule.isIdentifier(decl.name)) {
        const stmt = decl.parent.parent;
        if (tsModule.isVariableStatement(stmt) && hasExportModifier(stmt, tsModule)) {
            return {
                filePath: decl.getSourceFile().fileName,
                exportName: decl.name.text,
            };
        }
    }
    return undefined;
}
function resolveVariableDeclaration(id, checker, tsModule) {
    const symbol = checker.getSymbolAtLocation(id);
    const aliased = symbol && (symbol.flags & tsModule.SymbolFlags.Alias)
        ? checker.getAliasedSymbol(symbol)
        : symbol;
    const decl = aliased?.valueDeclaration ?? aliased?.declarations?.[0];
    return decl && tsModule.isVariableDeclaration(decl) ? decl : undefined;
}
function buildSyntheticModule(sourceFile, expr, exportName, tsModule, compilerOptions) {
    const parts = [];
    for (const stmt of sourceFile.statements) {
        if (tsModule.isImportDeclaration(stmt)) {
            parts.push(rewriteImport(stmt, sourceFile, tsModule, compilerOptions));
            continue;
        }
        if (stmt.end <= expr.getStart(sourceFile) && isReusableDeclaration(stmt, tsModule)) {
            parts.push(stmt.getText(sourceFile));
        }
    }
    parts.push(`export const ${exportName} = ${expr.getText(sourceFile)};`);
    return parts.join('\n\n');
}
function isReusableDeclaration(stmt, tsModule) {
    return (tsModule.isVariableStatement(stmt) ||
        tsModule.isFunctionDeclaration(stmt) ||
        tsModule.isClassDeclaration(stmt) ||
        tsModule.isEnumDeclaration(stmt) ||
        tsModule.isInterfaceDeclaration(stmt) ||
        tsModule.isTypeAliasDeclaration(stmt));
}
function rewriteImport(stmt, sourceFile, tsModule, compilerOptions) {
    const spec = stmt.moduleSpecifier;
    if (!tsModule.isStringLiteral(spec)) {
        return stmt.getText(sourceFile);
    }
    const resolved = tsModule.resolveModuleName(spec.text, sourceFile.fileName, compilerOptions, tsModule.sys).resolvedModule?.resolvedFileName;
    if (!resolved)
        return stmt.getText(sourceFile);
    if (resolved.includes('/node_modules/') || resolved.endsWith('.d.ts')) {
        return stmt.getText(sourceFile);
    }
    const text = stmt.getText(sourceFile);
    return text.replace(spec.getText(sourceFile), JSON.stringify(pathToFileURL(resolved).href));
}
function sourceMapForExpression(expr, sourceFile, tsModule) {
    const objectArg = findZObjectCall(expr, tsModule);
    if (objectArg)
        return buildSourceMapFromObjectLiteral(objectArg, sourceFile, tsModule);
    return buildRootSourceMap(expr, sourceFile);
}
function sourceMapForTarget(expr, sourceFile, checker, tsModule) {
    if (tsModule.isIdentifier(expr)) {
        const decl = resolveVariableDeclaration(expr, checker, tsModule);
        if (decl?.initializer) {
            return sourceMapForExpression(decl.initializer, decl.getSourceFile(), tsModule);
        }
    }
    return sourceMapForExpression(expr, sourceFile, tsModule);
}
function namedTarget(api, expression, sourceFile, tsModule, explicitName) {
    const { line } = sourceFile.getLineAndCharacterOfPosition(expression.getStart(sourceFile));
    const expr = skipParens(expression, tsModule);
    const suffix = explicitName ??
        (tsModule.isIdentifier(expr) ? expr.text : `inline:${line + 1}`);
    return {
        name: `${api}:${suffix}`,
        expression,
    };
}
function firstObjectArg(call, tsModule) {
    const arg = call.arguments[0];
    return arg && tsModule.isObjectLiteralExpression(arg) ? arg : undefined;
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
function skipParens(node, tsModule) {
    while (tsModule.isParenthesizedExpression(node))
        node = node.expression;
    return node;
}
function safeName(name) {
    return name.replace(/[^a-zA-Z0-9_]/g, '_');
}
//# sourceMappingURL=targets.js.map