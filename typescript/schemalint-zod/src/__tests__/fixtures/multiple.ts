import { z } from 'zod';

export const User = z.object({
  id: z.string(),
  name: z.string(),
});

export const Address = z.object({
  street: z.string(),
  city: z.string(),
  zip: z.string(),
});
