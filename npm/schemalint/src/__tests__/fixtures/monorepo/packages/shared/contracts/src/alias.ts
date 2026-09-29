import { z } from 'zod';

const Inner = z.object({ x: z.string() });
export const Wrapped = Inner;
