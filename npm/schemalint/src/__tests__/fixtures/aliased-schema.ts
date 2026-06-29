import { z } from 'zod';

export const AliasedSchema = z.object({
  aliased: z.string(),
});
