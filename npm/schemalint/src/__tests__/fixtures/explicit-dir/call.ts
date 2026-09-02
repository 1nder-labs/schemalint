import { generateObject } from 'ai';

import { TopSchema } from './top.ts';

export async function run() {
  return generateObject({ model: {} as never, schema: TopSchema, prompt: 'x' });
}
