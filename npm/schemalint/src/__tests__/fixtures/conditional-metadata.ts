import { generateObject } from 'ai';
import { zodTextFormat } from 'openai/helpers/zod';
import { z } from 'zod';

const First = z.object({ first: z.string() });
const Second = z.object({ second: z.string() });
const Selected = Math.random() > 0.5 ? First : Second;

generateObject(
  Math.random() > 0.5 ? { schema: First } : { schema: Second }
);
zodTextFormat(Selected, 'selected_schema');
zodTextFormat(First, Math.random() > 0.5 ? 'first_name' : 'second_name');

generateObject(Math.random() > 0.5 ? { schema: First } : { schema: First });
zodTextFormat(
  Math.random() > 0.5 ? First : First,
  Math.random() > 0.5 ? 'same_name' : 'same_name'
);
