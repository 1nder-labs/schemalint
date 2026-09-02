import { pathToFileURL } from 'node:url';
/**
 * Build a synthetic ES module containing only the transitive closure of
 * module-level declarations reachable from `seedExpr`, plus the import
 * declarations those kept statements (or `seedExpr` itself) actually
 * reference, ending in `export const <exportName> = <seedExpr text>;`.
 *
 * This replaces two failure modes seen on real projects: importing the
 * WHOLE declaring file (which runs every top-level side effect and every
 * import, including runtime-only schemes Node cannot load, e.g.
 * `cloudflare:workflows`), and copying every declaration/import
 * unconditionally (same problem, just duplicated into a temp file). Slicing
 * keeps only what the target expression's own dependency graph needs.
 *
 * ponytail: slicing only reaches into `sourceFile` itself — a kept import of
 * a LOCAL module is still loaded whole via `rewriteImport`. If a schema
 * helper module ever imports a runtime-only scheme, that import is a genuine
 * dependency and evaluation may still fail; cross-file slicing would be the
 * upgrade, not attempted here.
 */
export function buildSlicedModule(sourceFile, seedExpr, exportName, tsModule, compilerOptions) {
    const declByName = indexDeclarations(sourceFile, tsModule);
    const referenced = new Set();
    const kept = new Set();
    const queue = [];
    const enqueue = (names) => {
        for (const name of names) {
            if (referenced.has(name))
                continue;
            referenced.add(name);
            queue.push(name);
        }
    };
    enqueue(identifierNames(seedExpr, tsModule));
    while (queue.length > 0) {
        // Non-null: only names just pushed by `enqueue` land here, and the loop
        // condition already checked `queue.length > 0`.
        const name = queue.shift();
        for (const stmt of declByName.get(name) ?? []) {
            if (kept.has(stmt))
                continue;
            kept.add(stmt);
            enqueue(identifierNames(stmt, tsModule));
        }
    }
    const parts = [];
    for (const stmt of sourceFile.statements) {
        if (tsModule.isImportDeclaration(stmt)) {
            if (importIsReferenced(stmt, referenced, tsModule)) {
                parts.push(rewriteImport(stmt, sourceFile, tsModule, compilerOptions));
            }
            continue;
        }
        if (kept.has(stmt))
            parts.push(stmt.getText(sourceFile));
    }
    parts.push(`export const ${exportName} = ${seedExpr.getText(sourceFile)};`);
    return parts.join('\n\n');
}
/**
 * Index every module-level declaration by each name it binds. A name can map
 * to more than one statement (function overloads, merged interfaces), so
 * lookups return the full set to keep.
 */
function indexDeclarations(sourceFile, tsModule) {
    const map = new Map();
    for (const stmt of sourceFile.statements) {
        for (const name of boundNames(stmt, tsModule)) {
            const existing = map.get(name);
            if (existing)
                existing.push(stmt);
            else
                map.set(name, [stmt]);
        }
    }
    return map;
}
function boundNames(stmt, tsModule) {
    if (tsModule.isVariableStatement(stmt)) {
        return stmt.declarationList.declarations.flatMap((decl) => bindingNames(decl.name, tsModule));
    }
    if (tsModule.isFunctionDeclaration(stmt) ||
        tsModule.isClassDeclaration(stmt) ||
        tsModule.isEnumDeclaration(stmt) ||
        tsModule.isInterfaceDeclaration(stmt) ||
        tsModule.isTypeAliasDeclaration(stmt)) {
        return stmt.name ? [stmt.name.text] : [];
    }
    return [];
}
/** Every name a (possibly destructured, possibly nested) binding introduces. */
function bindingNames(name, tsModule) {
    if (tsModule.isIdentifier(name))
        return [name.text];
    return name.elements.flatMap((el) => tsModule.isOmittedExpression(el) ? [] : bindingNames(el.name, tsModule));
}
/**
 * Every identifier *referenced* within `node`, source order irrelevant.
 * Property keys (`{ onboard: ... }`, `a.onboard`, `method() {}`) are names,
 * not references: counting them would pull in an unrelated module-level
 * declaration that happens to share the key's spelling.
 */
function identifierNames(node, tsModule) {
    const names = new Set();
    const visit = (n) => {
        if (tsModule.isIdentifier(n)) {
            names.add(n.text);
            return;
        }
        if (tsModule.isPropertyAccessExpression(n)) {
            visit(n.expression);
            return;
        }
        if ((tsModule.isPropertyAssignment(n) ||
            tsModule.isMethodDeclaration(n) ||
            tsModule.isPropertyDeclaration(n) ||
            tsModule.isPropertySignature(n) ||
            tsModule.isMethodSignature(n)) &&
            !tsModule.isComputedPropertyName(n.name)) {
            tsModule.forEachChild(n, (child) => {
                if (child !== n.name)
                    visit(child);
            });
            return;
        }
        tsModule.forEachChild(n, visit);
    };
    visit(node);
    return names;
}
/** Whether any local binding this import introduces is in `referenced`. */
function importIsReferenced(stmt, referenced, tsModule) {
    const clause = stmt.importClause;
    if (!clause)
        return false; // side-effect-only import: no binding to reference
    if (clause.name && referenced.has(clause.name.text))
        return true;
    const bindings = clause.namedBindings;
    if (bindings && tsModule.isNamedImports(bindings)) {
        return bindings.elements.some((el) => referenced.has(el.name.text));
    }
    if (bindings && tsModule.isNamespaceImport(bindings)) {
        return referenced.has(bindings.name.text);
    }
    return false;
}
/**
 * Rewrite a local-module import's specifier to an absolute `file://` URL so
 * the synthetic module (written to a temp directory) can still resolve it.
 * node_modules and ambient `.d.ts` specifiers are left as-is: they resolve
 * the same way from any directory once `node_modules` is symlinked in
 * (see `linkNodeModules` in evaluate.ts).
 */
export function rewriteImport(stmt, sourceFile, tsModule, compilerOptions) {
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
/** A synthetic export name safe to use as a JS identifier. */
export function safeName(name) {
    return name.replace(/[^a-zA-Z0-9_]/g, '_');
}
//# sourceMappingURL=slice.js.map