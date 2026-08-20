import { pathToFileURL } from 'node:url';
import { buildRootSourceMap, buildSourceMapFromObjectLiteral, findZObjectCall, hasExportModifier, } from './discover_ast.js';
import { resolveVariableDeclaration, } from './target_resolution.js';
export function resolveTarget(target, checker, tsModule, compilerOptions) {
    const expr = unwrapLocalAlias(target.expression, checker, tsModule);
    const sourceFile = target.sourceFile;
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
/**
 * Follow a function-local `const schema = ...` to the expression it aliases.
 *
 * Only module-level declarations survive into the synthetic module (see
 * `isReusableDeclaration`), so a name bound inside a function body would be
 * emitted as an undefined reference. Its initializer is the real target, and
 * that initializer is either an inline expression or a module-level name the
 * synthetic module does hoist.
 */
function unwrapLocalAlias(expression, checker, tsModule) {
    let current = skipParens(expression, tsModule);
    // ponytail: bounded rather than cycle-tracked; `const a = b, b = a` is not
    // valid code, so the only chains here are short alias hops.
    for (let hop = 0; hop < 8 && tsModule.isIdentifier(current); hop++) {
        const decl = resolveVariableDeclaration(current, checker, tsModule);
        if (!decl?.initializer)
            break;
        const stmt = decl.parent.parent;
        const moduleLevel = tsModule.isVariableStatement(stmt) && tsModule.isSourceFile(stmt.parent);
        if (moduleLevel)
            break;
        current = skipParens(decl.initializer, tsModule);
    }
    return current;
}
function resolveExportedIdentifier(id, checker, tsModule) {
    const decl = resolveVariableDeclaration(id, checker, tsModule);
    if (!decl)
        return undefined;
    if (tsModule.isIdentifier(decl.name)) {
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
function buildSyntheticModule(sourceFile, expr, exportName, tsModule, compilerOptions) {
    const parts = [];
    // Identify the top-level statement that directly contains the target
    // expression so we can skip it (we replace it with the export below).
    const containingStmt = sourceFile.statements.find((stmt) => stmt.getStart(sourceFile) <= expr.getStart(sourceFile) &&
        expr.getEnd() <= stmt.getEnd());
    for (const stmt of sourceFile.statements) {
        if (tsModule.isImportDeclaration(stmt)) {
            parts.push(rewriteImport(stmt, sourceFile, tsModule, compilerOptions));
            continue;
        }
        // Include all reusable declarations except the one containing the target
        // expression. This allows the target to reference helpers declared either
        // before or after it in source order without a ReferenceError.
        if (stmt !== containingStmt && isReusableDeclaration(stmt, tsModule)) {
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
    // pathToFileURL produces a forward-slash percent-encoded file:// URL (Windows-safe);
    // JSON.stringify is the correct way to embed it as a JS string literal — not double-escaping.
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
function skipParens(node, tsModule) {
    while (tsModule.isParenthesizedExpression(node))
        node = node.expression;
    return node;
}
function safeName(name) {
    return name.replace(/[^a-zA-Z0-9_]/g, '_');
}
//# sourceMappingURL=target_emit.js.map