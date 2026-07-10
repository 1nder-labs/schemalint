import { zodOutputFormat } from '@anthropic-ai/sdk/helpers/zod';
import { z } from 'zod';

const Shared = z.object({ value: z.string() });

zodOutputFormat(Shared);
