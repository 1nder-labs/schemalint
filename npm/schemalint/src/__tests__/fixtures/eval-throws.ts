import { z } from 'zod';

// The throw is tied to the exported schema's own construction, not a stray
// top-level statement, so it survives AST slicing: `explode` is referenced
// directly from `Broken`'s initializer, so the closure walk in
// buildSlicedModule keeps it, and it runs when the sliced module imports.
function explode(): never {
  throw new Error('boom at import time');
}

export const Broken = z.object({ name: explode() });
