import { z } from 'zod';

export const ImportedSchema = z.object({
  imported: z.string(),
});
