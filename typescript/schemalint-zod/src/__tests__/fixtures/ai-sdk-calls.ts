import { z } from 'zod';
import { generateObject, streamObject, tool } from 'ai';

const LocalResult = z.object({
  title: z.string(),
  count: z.number(),
});

const VariableResult = z.object({
  variable: z.string(),
});

generateObject({
  schema: LocalResult,
});

const generateArgs = {
  schema: VariableResult,
};

generateObject(generateArgs);

const conditionalArgs = {
  schema: z.object({
    conditional: z.boolean(),
  }),
};

generateObject(Math.random() > -1 ? conditionalArgs : { schema: LocalResult });

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
