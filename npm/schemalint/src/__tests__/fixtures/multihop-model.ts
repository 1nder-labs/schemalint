import { Output } from 'ai';
import type { z } from 'zod';

// A params object threaded through several layers of wrapper. Each layer
// only forwards it — the real Zod schema arrives from whichever call site in
// multihop-calls.ts finally supplies one.
type Params<T> = { model: unknown; schema: z.ZodType<T>; prompt: string };

// Hop 1: reads `.schema` off its own parameter straight into the provider
// call. Resolves to a carrier on its own, same as the single-hop case.
async function attempt<T>(params: Params<T>, op: string) {
  return Output.object({ schema: params.schema });
}

// Hop 2, bare passthrough: forwards the whole `params` identifier unchanged.
export async function generateStructured<T>(params: Params<T>, op: string) {
  try {
    return await attempt(params, op);
  } catch {
    return attempt(params, op);
  }
}

// Hop 2, object-literal passthrough: re-wraps `params` via spread instead of
// forwarding the bare identifier.
export async function generateStructuredSpread<T>(
  params: Params<T>,
  op: string
) {
  return attempt({ ...params }, op);
}

// Hop 3: an outer layer wrapping the bare-passthrough wrapper above, so the
// schema has to survive three hops before it reaches Output.object.
export async function generateStructuredLogged<T>(
  params: Params<T>,
  op: string
) {
  return generateStructured(params, op);
}
