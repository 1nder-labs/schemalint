import { Middle } from './review-chain-middle.ts';

// Not exported: only the derived-name check can reach it.
const Final = Middle.extend({ note: Middle.shape.id.optional() });
export const parse = (input: unknown) => Final.parse(input);
