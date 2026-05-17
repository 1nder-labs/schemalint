import { z } from 'zod';
import { generateObject, streamObject, tool } from 'ai';

const LocalResult = z.object({
  title: z.string(),
  count: z.number(),
});

generateObject({
  schema: LocalResult,
});

streamObject({
  schema: z.object({
    chunk: z.string(),
  }),
});

tool({
  inputSchema: z.object({
    query: z.string(),
  }),
});
