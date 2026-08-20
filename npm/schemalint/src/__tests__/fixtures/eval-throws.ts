import { z } from 'zod';

// Module-level throw: AST discovery finds the export, runtime evaluation fails.
throw new Error('boom at import time');

export const Broken = z.object({ name: z.string() });
