import { z } from 'zod';
import {
  generateStructured,
  generateStructuredSpread,
  generateStructuredLogged,
} from './multihop-model.ts';

// Never passed to a provider — must stay undiscovered. Same guard as
// wrapper-calls.ts: carrier resolution must not widen into a repo-wide scan.
export const unrelatedSchema = z.object({ unrelated: z.string() });

const OnboardSchema = z.object({ onboard: z.string() });
const SpreadSchema = z.object({ spread: z.string() });
const LoggedSchema = z.object({ logged: z.string() });

// Reaches Output.object through the bare-passthrough wrapper (two hops).
export async function onboard() {
  return generateStructured(
    { model: {} as never, schema: OnboardSchema, prompt: 'x' },
    'onboard'
  );
}

// Reaches Output.object through the `{ ...params }` wrapper (two hops).
export async function spreadCall() {
  return generateStructuredSpread(
    { model: {} as never, schema: SpreadSchema, prompt: 'x' },
    'spread'
  );
}

// Reaches Output.object through three layers of wrapping.
export async function loggedCall() {
  return generateStructuredLogged(
    { model: {} as never, schema: LoggedSchema, prompt: 'x' },
    'logged'
  );
}
