/**
 * Runtime schema evaluation.
 *
 * Given a file path and export name, dynamically imports the user's TypeScript
 * file, accesses the exported Zod schema, and converts it to JSON Schema.
 *
 * Uses `zod-to-json-schema` by default, but detects Zod v4's native
 * `toJSONSchema()` method when available.
 */
/**
 * Dynamically import a user's TypeScript file and evaluate an exported
 * Zod schema to JSON Schema.
 *
 * Requires `tsx` for JIT compilation of TypeScript imports.
 */
export declare function evaluateSchema(filePath: string, exportName: string): Promise<Record<string, unknown>>;
//# sourceMappingURL=evaluate.d.ts.map