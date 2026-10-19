import { z } from 'zod';

const Inner = z.object({ aliased: z.string() });
export const Public = Inner;
