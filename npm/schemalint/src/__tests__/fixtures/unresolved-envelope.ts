import { z } from 'zod';
import { zodTextFormat } from 'openai/helpers/zod';

const name = process.env.RESPONSE_FORMAT_NAME ?? 'fallback';

zodTextFormat(z.object({ answer: z.string() }), name);
