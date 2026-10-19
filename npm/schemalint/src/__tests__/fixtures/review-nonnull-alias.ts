import { z } from 'zod';

const Inner = z.object({ nonNull: z.string() });
export const Public = Inner!;
