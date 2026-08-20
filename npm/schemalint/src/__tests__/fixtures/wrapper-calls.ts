import { generateObject } from 'ai';
import { z } from 'zod';

// Never passed to a provider — must stay undiscovered. This is the guard that
// carrier resolution does not degrade into a repo-wide Zod scan.
export const unrelatedSchema = z.object({ unrelated: z.string() });

const directSchema = z.object({ direct: z.string() });
const destructuredSchema = z.object({ destructured: z.string() });
const bareSchema = z.object({ bare: z.string() });
const renamedSchema = z.object({ renamed: z.string() });

// Shorthand at a direct call site, via a function-local alias.
export async function direct() {
  const schema = directSchema;
  return generateObject({ model: {} as never, schema, prompt: 'x' });
}

// Wrapper taking a destructured parameter, returned from a factory.
export function makeWrapper() {
  return async ({ schema, prompt }: { schema: z.ZodType; prompt: string }) =>
    generateObject({ model: {} as never, schema, prompt });
}

// Wrapper taking the schema as a whole parameter.
async function bareWrapper(schema: z.ZodType) {
  return generateObject({ model: {} as never, schema, prompt: 'x' });
}

// Wrapper whose destructured binding is renamed.
async function renamedWrapper({ schema: inner }: { schema: z.ZodType }) {
  return generateObject({ model: {} as never, schema: inner, prompt: 'x' });
}

// Wrapper reached only through a function-type annotation, the shape dependency
// injection produces: the arrow is never named at the call site.
type Compile = (input: { schema: z.ZodType; prompt: string }) => Promise<unknown>;

export function buildCompile(): Compile {
  return async ({ schema, prompt }) =>
    generateObject({ model: {} as never, schema, prompt });
}

const injectedSchema = z.object({ injected: z.string() });

export async function useInjected(compile: Compile) {
  return compile({ schema: injectedSchema, prompt: 'x' });
}

export async function callers() {
  await makeWrapper()({ schema: destructuredSchema, prompt: 'x' });
  await bareWrapper(bareSchema);
  await renamedWrapper({ schema: renamedSchema });
}
