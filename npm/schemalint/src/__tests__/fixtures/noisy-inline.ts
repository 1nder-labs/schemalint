import { WorkflowEntrypoint } from 'cloudflare:workflows';
import { generateObject } from 'ai';
import { z } from 'zod';

/**
 * Fixture: an INLINE call-site schema (path 2) in a file that also imports
 * an unloadable scheme and runs an unconditional side effect at module
 * scope, combined with the forward-reference case — the inline expression
 * references a helper declared AFTER the call site.
 */

// Never referenced by the inline schema below.
class NoisyEntrypoint extends WorkflowEntrypoint {}

// Unconditional at module scope and not tied to any declaration the schema
// references, so slicing drops this statement too.
if (globalThis.__schemalint_side_effect__ !== 'x') {
  throw new Error('must not load');
}

generateObject({
  schema: z.object({
    value: makeField(),
  }),
});

function makeField() {
  return z.string().min(1);
}
