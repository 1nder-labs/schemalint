export type Provider = 'openai' | 'anthropic';
export type ProviderCertainty = 'definitive' | 'inferred' | 'ambiguous';

export interface ProviderResolution {
  certainty: ProviderCertainty;
  provider?: Provider;
}

export interface TargetSpan {
  file: string;
  line: number;
  col: number;
}

export interface EnvelopeField {
  required: boolean;
  resolved: boolean;
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
  argumentAdapter(
    'openai/helpers/zod',
    'zodTextFormat',
    'openai.zodTextFormat',
    'openai',
    [requiredArgument('name', 1)]
  ),
  argumentAdapter(
    'openai/helpers/zod',
    'zodResponseFormat',
    'openai.zodResponseFormat',
    'openai',
    [requiredArgument('name', 1)]
  ),
  objectAdapter(
    'openai/helpers/zod',
    'zodFunction',
    'openai.zodFunction',
    ['parameters'],
    '2.0',
    [requiredProperty('name', 'name')],
    'openai'
  ),
  argumentAdapter(
    '@anthropic-ai/sdk/helpers/zod',
    'zodOutputFormat',
    'anthropic.zodOutputFormat',
    'anthropic'
  ),
  objectAdapter(
    '@anthropic-ai/sdk/helpers/zod',
    'betaZodTool',
    'anthropic.betaZodTool',
    ['inputSchema'],
    '2.0',
    [requiredProperty('name', 'name')],
    'anthropic'
  ),
];

const byImport = new Map(
  adapters.map((adapter) => [`${adapter.module}:${adapter.exportPath}`, adapter])
);

export function adapterFor(
  module: string,
  exportPath: string
): SdkAdapter | undefined {
  return byImport.get(`${module}:${exportPath}`);
}

function objectAdapter(
  module: string,
  exportPath: string,
  kind: string,
  properties: readonly string[],
  deprecatedRemoval?: '2.0',
  envelope: readonly EnvelopeSelector[] = [],
  provider?: Provider
): SdkAdapter {
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

function argumentAdapter(
  module: string,
  exportPath: string,
  kind: string,
  provider: Provider,
  envelope: readonly EnvelopeSelector[] = []
): SdkAdapter {
  return {
    module,
    exportPath,
    kind,
    provider,
    schema: { argument: 0 },
    envelope,
  };
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
