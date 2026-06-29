import { generateObject } from 'ai';
import { z } from 'zod';

interface ExtractorDefinition<T> {
  name: string;
  schema: z.ZodType<T>;
}

function makeExtractor<T>(def: ExtractorDefinition<T>) {
  return async () =>
    generateObject({
      schema: def.schema,
      schemaName: `${def.name}.extract`,
    });
}

const Extracted = z.object({
  extracted: z.string(),
});

makeExtractor({
  name: 'extractThing',
  schema: Extracted,
});

const InlineDefinition = {
  name: 'inlineThing',
  schema: z.object({
    inline: z.number(),
  }),
};

makeExtractor(InlineDefinition);
