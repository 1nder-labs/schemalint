import { z } from 'zod';
import { betaZodTool } from '@anthropic-ai/sdk/helpers/zod';

betaZodTool({
  name: 'translate',
  inputSchema: z.object({
    text: z.string(),
    target_language: z.string(),
  }),
});
