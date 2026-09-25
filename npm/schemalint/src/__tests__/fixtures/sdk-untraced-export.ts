import { z } from 'zod';

// Exported but never passed to a provider. In glob scope, the SDK import in
// the sibling file means this must not be linted as a fallback.
export const untracedSchema = z.object({ untraced: z.string() });
