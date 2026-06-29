import { z } from 'zod';
import { zodFunction, zodTextFormat } from 'openai/helpers/zod';
import { betaZodTool } from '@anthropic-ai/sdk/helpers/zod';

const ResponseSchema = z.object({
  answer: z.string(),
});

zodTextFormat(ResponseSchema, 'response');

zodFunction({
  name: 'lookup',
  parameters: z.object({
    id: z.string(),
  }),
});

betaZodTool({
  name: 'search',
  inputSchema: z.object({
    query: z.string(),
  }),
});
