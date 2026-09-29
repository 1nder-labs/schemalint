import { z } from 'zod';

// No tsconfig.json above this file: discovery synthesizes a default program.
export const Loose = z.object({ loose: z.boolean() });
