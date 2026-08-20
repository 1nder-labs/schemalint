import { adapterFor, hasAdapterPrefix, } from './sdk_adapters.js';
export function collectTargetImports(sourceFile, tsModule) {
    const imports = {
        functions: new Map(),
        objects: new Map(),
        namespaces: new Map(),
    };
    for (const stmt of sourceFile.statements) {
        if (!tsModule.isImportDeclaration(stmt))
            continue;
        const spec = stmt.moduleSpecifier;
        const clause = stmt.importClause;
        if (!tsModule.isStringLiteral(spec) || !clause)
            continue;
        const module = spec.text;
        const bindings = clause.namedBindings;
        if (bindings && tsModule.isNamespaceImport(bindings)) {
            imports.namespaces.set(bindings.name.text, module);
            continue;
        }
        if (!bindings || !tsModule.isNamedImports(bindings))
            continue;
        for (const element of bindings.elements) {
            const importedName = element.propertyName?.text ?? element.name.text;
            const localName = element.name.text;
            const adapter = adapterFor(module, importedName);
            if (adapter) {
                imports.functions.set(localName, adapter);
            }
            else if (hasAdapterPrefix(module, importedName)) {
                imports.objects.set(localName, { module, exportPath: importedName });
            }
        }
    }
    return imports;
}
export function resolveTargetAdapter(expression, imports, tsModule) {
    if (tsModule.isIdentifier(expression)) {
        return imports.functions.get(expression.text);
    }
    const path = propertyPath(expression, tsModule);
    if (!path || path.length < 2)
        return undefined;
    const [root, ...members] = path;
    const namespaceModule = imports.namespaces.get(root);
    if (namespaceModule)
        return adapterFor(namespaceModule, members.join('.'));
    const object = imports.objects.get(root);
    if (!object)
        return undefined;
    return adapterFor(object.module, `${object.exportPath}.${members.join('.')}`);
}
function propertyPath(expression, tsModule) {
    if (tsModule.isIdentifier(expression))
        return [expression.text];
    if (!tsModule.isPropertyAccessExpression(expression))
        return undefined;
    const parent = propertyPath(expression.expression, tsModule);
    return parent ? [...parent, expression.name.text] : undefined;
}
//# sourceMappingURL=target_imports.js.map