import { z } from 'zod';

// The outer chain uses a parameter; the schema it extends does not.
export function make(extra: z.ZodRawShape) {
  return z.object({ fixedPart: z.string().optional() }).extend(extra);
}
