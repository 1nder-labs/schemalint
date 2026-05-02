import { z } from 'zod';

export const Order = z.object({
  id: z.string(),
  customer: z.object({
    email: z.string(),
    address: z.object({
      street: z.string(),
      city: z.string(),
    }),
  }),
});
