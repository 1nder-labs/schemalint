import { z } from 'zod';

// Tests source-map building for non-identifier property name syntaxes:
//   - string-literal key:                  { 'email': ... }
//   - computed key with string literal:    { ['name']: ... }
//   - computed key with dynamic expr:      { [k]: ... }  — skipped (no static name)
//   - spread:                              { ...base }   — skipped
const k = 'dynamic';
const base = { extra: z.string() };

export const MixedKeySchema = z.object({
  'email': z.string(),
  ['name']: z.string(),
  [k]: z.number(),
  ...base,
});
