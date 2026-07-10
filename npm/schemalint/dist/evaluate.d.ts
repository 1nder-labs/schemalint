/**
 * Runtime schema evaluation.
 *
 * Given a file path and export name, dynamically imports the user's TypeScript
 * file, accesses the exported Zod schema, and converts it to JSON Schema.
 *
 * Uses Zod v4's package-level `toJSONSchema()` API for v4 and Mini schemas,
 * and `zod-to-json-schema` for Zod v3.
 */
/**
 * Dynamically import a user's TypeScript file and evaluate an exported
 * Zod schema to JSON Schema.
 *
 * Requires `tsx` for JIT compilation of TypeScript imports.
 */
export declare function evaluateSchema(filePath: string, exportName: string): Promise<Record<string, unknown>>;
export declare function evaluateSyntheticSchema(source: string, exportName: string, baseFilePath: string): Promise<Record<string, unknown>>;
//# sourceMappingURL=evaluate.d.ts.map