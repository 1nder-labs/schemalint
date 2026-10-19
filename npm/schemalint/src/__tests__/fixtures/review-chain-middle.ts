import { Base } from './review-chain-base.ts';

// No zod import here: a schema derived purely from an imported one.
export const Middle = Base.pick({ id: true });
