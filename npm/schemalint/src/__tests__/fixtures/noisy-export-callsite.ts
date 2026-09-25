import { generateObject } from 'ai';

import { NoisySchema } from './noisy-export.ts';

// Provider call-site target resolving to an identifier EXPORTED FROM
// ANOTHER FILE that also imports an unloadable scheme and runs an
// unconditional side effect at module scope. Path 1 (cross-file exported
// identifier) must slice `noisy-export.ts` in its own context, not import
// it whole.
generateObject({
  schema: NoisySchema,
});
