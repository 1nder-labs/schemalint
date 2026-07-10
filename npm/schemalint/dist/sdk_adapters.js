const adapters = [
    objectAdapter('ai', 'generateObject', 'ai.generateObject', ['schema'], '2.0', [
        optionalProperty('name', 'schemaName'),
        optionalProperty('description', 'schemaDescription'),
    ]),
    objectAdapter('ai', 'streamObject', 'ai.streamObject', ['schema'], '2.0', [
        optionalProperty('name', 'schemaName'),
        optionalProperty('description', 'schemaDescription'),
    ]),
    objectAdapter('ai', 'Output.object', 'ai.Output.object', ['schema'], undefined, [
        optionalProperty('name', 'name'),
        optionalProperty('description', 'description'),
    ]),
    objectAdapter('ai', 'Output.array', 'ai.Output.array', ['element'], undefined, [
        optionalProperty('name', 'name'),
        optionalProperty('description', 'description'),
    ]),
    objectAdapter('ai', 'tool', 'ai.tool', ['inputSchema', 'parameters'], '2.0', [
        optionalProperty('description', 'description'),
    ]),
    objectAdapter('ai', 'dynamicTool', 'ai.dynamicTool', ['inputSchema'], undefined, [
        optionalProperty('description', 'description'),
    ]),
    argumentAdapter('openai/helpers/zod', 'zodTextFormat', 'openai.zodTextFormat', 'openai', [requiredArgument('name', 1)]),
    argumentAdapter('openai/helpers/zod', 'zodResponseFormat', 'openai.zodResponseFormat', 'openai', [requiredArgument('name', 1)]),
    objectAdapter('openai/helpers/zod', 'zodFunction', 'openai.zodFunction', ['parameters'], '2.0', [requiredProperty('name', 'name')], 'openai'),
    argumentAdapter('@anthropic-ai/sdk/helpers/zod', 'zodOutputFormat', 'anthropic.zodOutputFormat', 'anthropic'),
    objectAdapter('@anthropic-ai/sdk/helpers/zod', 'betaZodTool', 'anthropic.betaZodTool', ['inputSchema'], '2.0', [requiredProperty('name', 'name')], 'anthropic'),
];
const byImport = new Map(adapters.map((adapter) => [`${adapter.module}:${adapter.exportPath}`, adapter]));
export function adapterFor(module, exportPath) {
    return byImport.get(`${module}:${exportPath}`);
}
function objectAdapter(module, exportPath, kind, properties, deprecatedRemoval, envelope = [], provider) {
    return {
        module,
        exportPath,
        kind,
        provider,
        schema: { argument: 0, properties },
        envelope,
        deprecatedRemoval,
    };
}
function argumentAdapter(module, exportPath, kind, provider, envelope = []) {
    return {
        module,
        exportPath,
        kind,
        provider,
        schema: { argument: 0 },
        envelope,
    };
}
function requiredArgument(name, argument) {
    return { name, required: true, argument };
}
function requiredProperty(name, property) {
    return { name, required: true, argument: 0, property };
}
function optionalProperty(name, property) {
    return { name, required: false, argument: 0, property };
}
//# sourceMappingURL=sdk_adapters.js.map