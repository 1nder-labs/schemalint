import { z } from 'zod';

// Tests RFC 6901 pointer escaping in the source-map builder: a property
// name containing '/' (or '~') must produce an escaped pointer that still
// matches the pointer the Rust normalizer builds for the same schema.
export const SlashKeySchema = z.object({
  'a/b': z.string(),
  'c~d': z.string(),
});
