import { z } from 'zod';

// `Inner` is a separately declared const, not an inline `z.object()`
// literal at the `a:` call site. `buildSourceMapFromObjectLiteral` only
// recurses into an inline literal, so it maps `/properties/a` (the
// property assignment itself) but never anything inside `Inner`. Kept
// non-exported so discovery finds only `Outer`.
const Inner = z.object({
  site: z.string(),
});

export const Outer = z.object({
  a: Inner,
});
