import { z } from 'zod';

// Reachable only through the exported-schema fallback.
export const fallbackSchema = z.object({ fallback: z.string() });
