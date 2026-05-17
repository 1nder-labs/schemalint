export const OUTPUT_CALLS = new Set(['generateObject', 'streamObject']);
export const TOOL_CALLS = new Set(['tool', 'betaZodTool', 'zodFunction']);
export const FORMAT_CALLS = new Set(['zodTextFormat', 'zodResponseFormat']);
export function collectTargetImports(sourceFile, tsModule) {
    const imports = {
        outputCalls: new Set(),
        toolCalls: new Set(),
        formatCalls: new Set(),
        outputObjects: new Set(),
        aiNamespaces: new Set(),
        helperNamespaces: new Set(),
    };
    for (const stmt of sourceFile.statements) {
        if (!tsModule.isImportDeclaration(stmt))
            continue;
        const spec = stmt.moduleSpecifier;
        if (!tsModule.isStringLiteral(spec))
            continue;
        const clause = stmt.importClause;
        if (!clause)
            continue;
        const mod = spec.text;
        const isAiModule = mod === 'ai';
        const isOpenAiHelper = mod === 'openai/helpers/zod';
        const isAnthropicHelper = mod === '@anthropic-ai/sdk/helpers/zod';
        if (!isAiModule && !isOpenAiHelper && !isAnthropicHelper)
            continue;
        const bindings = clause.namedBindings;
        if (bindings && tsModule.isNamespaceImport(bindings)) {
            if (isAiModule)
                imports.aiNamespaces.add(bindings.name.text);
            if (isOpenAiHelper || isAnthropicHelper) {
                imports.helperNamespaces.add(bindings.name.text);
            }
            continue;
        }
        if (!bindings || !tsModule.isNamedImports(bindings))
            continue;
        for (const element of bindings.elements) {
            const importedName = element.propertyName?.text ?? element.name.text;
            const localName = element.name.text;
            if (isAiModule)
                addAiImport(imports, importedName, localName);
            if (isOpenAiHelper || isAnthropicHelper) {
                addHelperImport(imports, importedName, localName);
            }
        }
    }
    return imports;
}
export function isProviderCall(ref, imports) {
    if (ref.receiver) {
        if (imports.aiNamespaces.has(ref.receiver)) {
            return OUTPUT_CALLS.has(ref.name) || ref.name === 'tool';
        }
        if (imports.helperNamespaces.has(ref.receiver)) {
            return FORMAT_CALLS.has(ref.name) || TOOL_CALLS.has(ref.name);
        }
        return ref.name === 'object' && imports.outputObjects.has(ref.receiver);
    }
    return (imports.outputCalls.has(ref.name) ||
        imports.toolCalls.has(ref.name) ||
        imports.formatCalls.has(ref.name));
}
function addAiImport(imports, importedName, localName) {
    if (OUTPUT_CALLS.has(importedName))
        imports.outputCalls.add(localName);
    if (importedName === 'tool')
        imports.toolCalls.add(localName);
    if (importedName === 'Output')
        imports.outputObjects.add(localName);
}
function addHelperImport(imports, importedName, localName) {
    if (FORMAT_CALLS.has(importedName))
        imports.formatCalls.add(localName);
    if (TOOL_CALLS.has(importedName))
        imports.toolCalls.add(localName);
}
//# sourceMappingURL=target_imports.js.map