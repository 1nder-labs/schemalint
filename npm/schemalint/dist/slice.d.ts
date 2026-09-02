import type * as ts from 'typescript';
/**
 * Build a synthetic ES module containing only the transitive closure of
 * module-level declarations reachable from `seedExpr`, plus the import
 * declarations those kept statements (or `seedExpr` itself) actually
 * reference, ending in `export const <exportName> = <seedExpr text>;`.
 *
 * This replaces two failure modes seen on real projects: importing the
 * WHOLE declaring file (which runs every top-level side effect and every
 * import, including runtime-only schemes Node cannot load, e.g.
 * `cloudflare:workflows`), and copying every declaration/import
 * unconditionally (same problem, just duplicated into a temp file). Slicing
 * keeps only what the target expression's own dependency graph needs.
 *
 * ponytail: slicing only reaches into `sourceFile` itself — a kept import of
 * a LOCAL module is still loaded whole via `rewriteImport`. If a schema
 * helper module ever imports a runtime-only scheme, that import is a genuine
 * dependency and evaluation may still fail; cross-file slicing would be the
 * upgrade, not attempted here.
 */
export declare function buildSlicedModule(sourceFile: ts.SourceFile, seedExpr: ts.Expression, exportName: string, tsModule: typeof ts, compilerOptions: ts.CompilerOptions): string;
/**
 * Rewrite a local-module import's specifier to an absolute `file://` URL so
 * the synthetic module (written to a temp directory) can still resolve it.
 * node_modules and ambient `.d.ts` specifiers are left as-is: they resolve
 * the same way from any directory once `node_modules` is symlinked in
 * (see `linkNodeModules` in evaluate.ts).
 */
export declare function rewriteImport(stmt: ts.ImportDeclaration, sourceFile: ts.SourceFile, tsModule: typeof ts, compilerOptions: ts.CompilerOptions): string;
/** A synthetic export name safe to use as a JS identifier. */
export declare function safeName(name: string): string;
//# sourceMappingURL=slice.d.ts.map