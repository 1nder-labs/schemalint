import { generateObject } from 'ai';
import { z } from 'zod';

// A class-held schema: the call site resolves to `this.schema`, which cannot be
// evaluated at module scope. The target exists but yields no model.
export class Runner {
  schema = z.object({ held: z.string() });
  async run() {
    const schema = this.schema;
    return generateObject({ model: {} as never, schema, prompt: 'x' });
  }
}
