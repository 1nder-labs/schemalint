import { z } from 'zod';
import * as zod from 'zod';

// Recognised by type, however it is spelled, exported or not.
const Declared = z.strictObject({ declared: z.string() });
export const Exported = zod.object({ exported: z.string() });

// The factory body uses its parameter, so only the call builds a schema.
const make = (about: string) => z.object({ made: z.string().describe(about) });
const Made = make('made');

function register(name: string, schema: z.ZodType) {
  return { name, schema };
}
export const registry = [register('inline', z.object({ inline: z.string() }).describe('x'))];

// Not an object schema: a building block, not a root.
const Status = z.enum(['a', 'b']);

// Built from run-time input inside a function: cannot be evaluated, so not a root.
export function handler(input: { fields: string[] }) {
  return z.object(Object.fromEntries(input.fields.map((field) => [field, z.string()])));
}

// Inline in a function body but built from nothing at run time: a root, the
// shape `generateObject({ schema: z.object({...}) })` takes in any SDK.
export async function run(send: (schema: z.ZodType) => Promise<void>) {
  await send(z.object({ inFunction: z.string() }));
}

// The nested object is linted only as part of its root.
const Outer = z.object({ nested: z.object({ deep: z.string() }) });

export const used = [Declared, Made, Status, Outer];
