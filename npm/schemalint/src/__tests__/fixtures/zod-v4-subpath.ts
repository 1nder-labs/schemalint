import * as z from 'zod/v4';

// Zod 3.25's root is still v3; this schema is v4 through the subpath.
export const ViaSubpath = z.object({ viaSubpath: z.string() });
