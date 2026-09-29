import { z } from 'zod';

// Matched by the tsconfig "exclude": skipped for directory and glob scopes.
export const Gen = z.object({ gen: z.string() });
