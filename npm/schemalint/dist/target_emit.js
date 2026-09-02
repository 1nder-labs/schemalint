import { buildRootSourceMap, buildSourceMapFromObjectLiteral, findZObjectCall, hasExportModifier, } from './discover_ast.js';
import { buildSlicedModule, safeName } from './slice.js';
import { resolveVariableDeclaration, unwrapExpression, } from './static_expression.js';
export function declarationKey(file, name) {
    return `${file}#${name}`;
}
export function resolveTarget(target, checker, tsModule, compilerOptions) {
    const expr = unwrapLocalAlias(target.expression, checker, tsModule);
    const sourceFile = target.sourceFile;
    const sourceMap = sourceMapForTarget(expr, sourceFile, checker, tsModule);
    if (tsModule.isIdentifier(expr)) {
        const exported = resolveExportedIdentifier(expr, checker, tsModule);
        if (exported) {
            const exportName = `__schemalint_target_${safeName(target.name)}`;
            return {
                name: target.name,
                filePath: exported.sourceFile.fileName,
                exportName,
                sourceMap,
                canonicalKind: target.metadata.canonicalKind,
                provider: target.metadata.provider,
                envelope: target.metadata.envelope,
                usageSpan: target.metadata.usageSpan,
                declaration: declarationKey(exported.sourceFile.fileName, exported.identifier.text),
                // Sliced in the DECLARING file's context (its imports, its
                // declarations), not the call site's — `exported.identifier` is the
                // declaration's own name node, physically part of that file.
                syntheticSource: buildSlicedModule(exported.sourceFile, exported.identifier, exportName, tsModule, compilerOptions),
            };
        }
    }
    const exportName = `__schemalint_target_${safeName(target.name)}`;
    return {
        name: target.name,
        filePath: sourceFile.fileName,
        exportName,
        sourceMap,
        canonicalKind: target.metadata.canonicalKind,
        provider: target.metadata.provider,
        envelope: target.metadata.envelope,
        usageSpan: target.metadata.usageSpan,
        syntheticSource: buildSlicedModule(sourceFile, expr, exportName, tsModule, compilerOptions),
    };
}
/**
 * Follow a function-local `const schema = ...` to the expression it aliases.
 *
 * Only module-level declarations survive into the synthetic module (see
 * `buildSlicedModule` in slice.ts), so a name bound inside a function body
 * would be emitted as an undefined reference. Its initializer is the real
 * target, and that initializer is either an inline expression or a
 * module-level name the synthetic module does hoist.
 */
function unwrapLocalAlias(expression, checker, tsModule) {
    let current = unwrapExpression(expression, tsModule);
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
        current = unwrapExpression(decl.initializer, tsModule);
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
            return { sourceFile: decl.getSourceFile(), identifier: decl.name };
        }
    }
    return undefined;
}
/**
 * Merge the root ('') entry with any property-level entries so every model's
 * source_map has a '' key pointing at the schema expression's own line, even
 * when it resolves to a `z.object({...})` call whose property map would
 * otherwise be the only content.
 */
function sourceMapForExpression(expr, sourceFile, tsModule) {
    const root = buildRootSourceMap(expr, sourceFile);
    const objectArg = findZObjectCall(expr, tsModule);
    if (!objectArg)
        return root;
    return {
        ...root,
        ...buildSourceMapFromObjectLiteral(objectArg, sourceFile, tsModule),
    };
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
//# sourceMappingURL=target_emit.js.map