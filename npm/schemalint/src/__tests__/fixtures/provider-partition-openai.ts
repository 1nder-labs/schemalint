import { Output } from 'ai';
import { zodTextFormat } from 'openai/helpers/zod';
import { z } from 'zod';

const Shared = z.object({ value: z.string() });

zodTextFormat(Shared, 'openai_result');
Output.object({ schema: Shared });
