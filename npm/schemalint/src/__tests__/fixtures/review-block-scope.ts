import { z } from 'zod';

const Schema = z.object({ outer: z.string() });
{
  const Schema = z.object({ blockScoped: z.string().optional() });
  void Schema;
}
export const used = Schema;
