const adapters = [
    {
        module: 'ai',
        exportPath: 'generateObject',
        kind: 'ai.generateObject',
        schema: { argument: 0, properties: ['schema'] },
        envelope: [
            optionalProperty('name', 'schemaName'),
            optionalProperty('description', 'schemaDescription'),
        ],
        deprecatedRemoval: '2.0',
    },
    {
        module: 'ai',
        exportPath: 'streamObject',
        kind: 'ai.streamObject',
        schema: { argument: 0, properties: ['schema'] },
        envelope: [
            optionalProperty('name', 'schemaName'),
            optionalProperty('description', 'schemaDescription'),
        ],
        deprecatedRemoval: '2.0',
    },
    {
        module: 'ai',
        exportPath: 'Output.object',
        kind: 'ai.Output.object',
        schema: { argument: 0, properties: ['schema'] },
        envelope: [
            optionalProperty('name', 'name'),
            optionalProperty('description', 'description'),
        ],
    },
    {
        module: 'ai',
        exportPath: 'Output.array',
        kind: 'ai.Output.array',
        schema: { argument: 0, properties: ['element'] },
        envelope: [
            optionalProperty('name', 'name'),
            optionalProperty('description', 'description'),
        ],
    },
    {
        module: 'ai',
        exportPath: 'tool',
        kind: 'ai.tool',
        schema: { argument: 0, properties: ['inputSchema', 'parameters'] },
        envelope: [optionalProperty('description', 'description')],
        deprecatedRemoval: '2.0',
    },
    {
        module: 'ai',
        exportPath: 'dynamicTool',
        kind: 'ai.dynamicTool',
        schema: { argument: 0, properties: ['inputSchema'] },
        envelope: [optionalProperty('description', 'description')],
    },
    {
        module: 'openai/helpers/zod',
        exportPath: 'zodTextFormat',
        kind: 'openai.zodTextFormat',
        provider: 'openai',
        schema: { argument: 0 },
        envelope: [requiredArgument('name', 1)],
    },
    {
        module: 'openai/helpers/zod',
        exportPath: 'zodResponseFormat',
        kind: 'openai.zodResponseFormat',
        provider: 'openai',
        schema: { argument: 0 },
        envelope: [requiredArgument('name', 1)],
    },
    {
        module: 'openai/helpers/zod',
        exportPath: 'zodFunction',
        kind: 'openai.zodFunction',
        provider: 'openai',
        schema: { argument: 0, properties: ['parameters'] },
        envelope: [requiredProperty('name', 'name')],
        deprecatedRemoval: '2.0',
    },
    {
        module: '@anthropic-ai/sdk/helpers/zod',
        exportPath: 'zodOutputFormat',
        kind: 'anthropic.zodOutputFormat',
        provider: 'anthropic',
        schema: { argument: 0 },
        envelope: [],
    },
    {
        module: '@anthropic-ai/sdk/helpers/beta/zod',
        exportPath: 'betaZodTool',
        kind: 'anthropic.betaZodTool',
        provider: 'anthropic',
        schema: { argument: 0, properties: ['inputSchema'] },
        envelope: [requiredProperty('name', 'name')],
        deprecatedRemoval: '2.0',
    },
];
const byImport = new Map(adapters.map((adapter) => [`${adapter.module}:${adapter.exportPath}`, adapter]));
export function adapterFor(module, exportPath) {
    return byImport.get(`${module}:${exportPath}`);
}
export function hasAdapterPrefix(module, exportPath) {
    const prefix = `${exportPath}.`;
    return adapters.some((adapter) => adapter.module === module && adapter.exportPath.startsWith(prefix));
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