import { WorkflowEntrypoint } from 'cloudflare:workflows';
import { z } from 'zod';

/**
 * Fixture: a file that exports a schema alongside content real projects have
 * that has nothing to do with the schema — a runtime-only import Node's
 * default ESM loader cannot resolve ("Only URLs with a scheme in: file,
 * data, and node are supported"), and an unconditional module-level side
 * effect. Neither is referenced by `NoisySchema`, so AST slicing must drop
 * both from the synthetic module it evaluates.
 */

// Never referenced by NoisySchema below — proves the unloadable import is
// excluded from the slice rather than dragging the whole file in.
class NoisyEntrypoint extends WorkflowEntrypoint {}

// Unconditional at module scope and not tied to any declaration the schema
// references, so slicing drops this statement too.
if (globalThis.__schemalint_side_effect__ !== 'x') {
  throw new Error('must not load');
}

export const NoisySchema = z.object({
  clean: z.string(),
});
