import { z } from 'zod';
import * as aiSdk from 'ai';
import { Output as Result, dynamicTool as makeDynamicTool } from 'ai';
import { zodTextFormat as openaiFormat } from 'openai/helpers/zod';
import * as anthropicZod from '@anthropic-ai/sdk/helpers/zod';

const Shared = z.object({ shared: z.string() });

openaiFormat(Shared, 'open_response');
anthropicZod.zodOutputFormat(Shared);

aiSdk.Output.object({
  name: 'object_result',
  description: 'one object',
  schema: Shared,
});

Result.array({
  name: 'array_result',
  element: z.object({ item: z.string() }),
});

makeDynamicTool({
  description: 'look something up',
  inputSchema: z.object({ query: z.string() }),
});
