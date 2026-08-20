export type Provider = 'openai' | 'anthropic';

export type ProviderResolution =
  | { certainty: 'definitive' | 'inferred'; provider: Provider }
  | { certainty: 'ambiguous'; provider?: never };

export interface TargetSpan {
  file: string;
  line: number;
  col: number;
}

export interface EnvelopeField {
  required: boolean;
  span: TargetSpan;
  value?: string;
}

interface PropertySelector {
  argument: number;
  properties: readonly string[];
}

interface ArgumentSelector {
  argument: number;
}

export interface EnvelopeSelector {
  name: string;
  required: boolean;
  argument: number;
  property?: string;
}

export interface SdkAdapter {
  module: string;
  exportPath: string;
  kind: string;
  provider?: Provider;
  schema: PropertySelector | ArgumentSelector;
  envelope: readonly EnvelopeSelector[];
  deprecatedRemoval?: '2.0';
}

const adapters: readonly SdkAdapter[] = [
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
] satisfies readonly SdkAdapter[];

const byImport = new Map(
  adapters.map((adapter) => [`${adapter.module}:${adapter.exportPath}`, adapter])
);

export function adapterFor(
  module: string,
  exportPath: string
): SdkAdapter | undefined {
  return byImport.get(`${module}:${exportPath}`);
}

export function hasAdapterPrefix(module: string, exportPath: string): boolean {
  const prefix = `${exportPath}.`;
  return adapters.some(
    (adapter) =>
      adapter.module === module && adapter.exportPath.startsWith(prefix)
  );
}

function requiredArgument(name: string, argument: number): EnvelopeSelector {
  return { name, required: true, argument };
}

function requiredProperty(name: string, property: string): EnvelopeSelector {
  return { name, required: true, argument: 0, property };
}

function optionalProperty(name: string, property: string): EnvelopeSelector {
  return { name, required: false, argument: 0, property };
}
