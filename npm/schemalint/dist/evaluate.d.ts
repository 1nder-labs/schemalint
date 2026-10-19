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
/**
 * One temp root for a whole run. Each distinct `node_modules` is symlinked
 * once into its own subdirectory, so synthetic modules written there resolve
 * packages the way the declaring file would.
 */
export interface EvalWorkspace {
    /** A fresh `.ts` path in a directory that resolves `baseFilePath`'s packages. */
    newModulePath(baseFilePath: string): Promise<string>;
    dispose(): Promise<void>;
}
export declare function createEvalWorkspace(): Promise<EvalWorkspace>;
export type ExportOutcome = {
    ok: true;
    schema: Record<string, unknown>;
} | {
    ok: false;
    error: unknown;
};
/**
 * Evaluate several exports of one synthetic module with a single write and
 * import. Each export converts on its own, so one conversion error stays with
 * its export. A throw while importing the module affects every export, so it
 * rejects instead; callers retry the exports individually.
 */
export declare function evaluateSyntheticExports(source: string, exportNames: readonly string[], baseFilePath: string, workspace: EvalWorkspace): Promise<ExportOutcome[]>;
export declare function evaluateSyntheticSchema(source: string, exportName: string, baseFilePath: string, sharedWorkspace?: EvalWorkspace): Promise<Record<string, unknown>>;
/**
 * Redirect stdout to stderr during evaluation so user-code console.log()
 * calls don't corrupt the JSON-RPC protocol channel. Reentrant and safe for
 * overlapping calls: stdout is restored when the last one finishes.
 */
export declare function withStdoutRedirect<T>(fn: () => Promise<T>): Promise<T>;
//# sourceMappingURL=evaluate.d.ts.map