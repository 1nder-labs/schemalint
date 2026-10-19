import { z } from 'zod';

export const First = z.object({ a: z.string() });

export const Second = z.object({ b: z.number() });
