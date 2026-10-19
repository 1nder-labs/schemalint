import { z } from 'zod';

function explode(): never {
  throw new Error('boom at import time');
}

export const Good = z.object({ name: z.string() });

export const Broken = z.object({ name: explode() });
