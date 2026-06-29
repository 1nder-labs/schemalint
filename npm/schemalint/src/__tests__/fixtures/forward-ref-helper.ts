/**
 * Fixture: inline schema expression referencing a helper function that is
 * declared AFTER the call site in source order. The synthetic module builder
 * must include all module-level declarations, not just the ones that appear
 * before the target expression.
 */
import { z } from 'zod';
import { generateObject } from 'ai';

// The call-site schema is inline; it references `makeField` which is declared
// below it.  Without the fix, the synthetic module omits `makeField` and
// evaluation throws ReferenceError.
generateObject({
  schema: z.object({
    value: makeField(),
  }),
});

function makeField() {
  return z.string().min(1);
}
