import { generateObject, Output } from 'ai';
import { ImportedSchema } from './schema-defs.ts';
import { AliasedSchema } from '#schemas';

declare function generateText(args: { output: unknown }): unknown;

generateObject({
  schema: ImportedSchema,
});

generateText({
  output: Output.object({
    schema: AliasedSchema,
  }),
});
