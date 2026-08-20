export type Provider = 'openai' | 'anthropic';
export type ProviderResolution = {
    certainty: 'definitive' | 'inferred';
    provider: Provider;
} | {
    certainty: 'ambiguous';
    provider?: never;
};
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
export declare function adapterFor(module: string, exportPath: string): SdkAdapter | undefined;
export declare function hasAdapterPrefix(module: string, exportPath: string): boolean;
export {};
//# sourceMappingURL=sdk_adapters.d.ts.map