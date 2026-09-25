import type * as ts from 'typescript';
/**
 * Syntactic alias data for carrier call sites. `edges` maps a bound name to
 * every name it may stand for (`const bound = source`,
 * `import { source as bound }`, `{ bound: source }`, `obj.bound = source`,
 * `export { source as bound }`, `bound: typeof source`, ...). `dynamic`
 * names are re-exported under a caller-chosen name (`export default x`,
 * `export = x`, `module.exports = x`), so a carrier reachable through one
 * is invocable under anything and cannot be name-filtered.
 */
export interface InvocationAliases {
    edges: Map<string, Set<string>>;
    dynamic: Set<string>;
}
/**
 * Names a carrier can be invoked under at a call site — the base set before
 * alias expansion. Undefined when no name set is a safe superset: the
 * carrier has no visible binding name (factory return, inline callback), is
 * exported under a caller-chosen name (`export default`, `export =`), or
 * was written against a function type — call sites then invoke it through
 * a parameter or variable of that type under any name at all.
 */
export declare function carrierBaseNames(fn: ts.FunctionLikeDeclaration, contextualSignatures: ReadonlySet<ts.Node>, tsModule: typeof ts): Set<string> | undefined;
/**
 * One syntax-only pass over the selected files collecting carrier
 * invocation aliases. Over-collecting is always safe — a wrong edge only
 * admits a call to the checker, which remains the source of truth.
 */
export declare function collectInvocationAliases(program: ts.Program, fileSet: ReadonlySet<string>, tsModule: typeof ts): InvocationAliases;
/**
 * Chase alias edges to a fixed point. Undefined — the carrier cannot be
 * name-filtered — when a reachable name escapes through a dynamic export
 * (`export default`, `export =`, `module.exports =`) and is therefore
 * invocable under an arbitrary name.
 */
export declare function expandCarrierNames(base: ReadonlySet<string>, aliases: InvocationAliases): Set<string> | undefined;
/**
 * The name a call invokes, when statically visible: `f()` → `f`,
 * `a.b.c()` → `c` (covers `this.f`, `super.f`, namespace calls). Any other
 * callee shape — `a[i]()`, `(f)()`, `f()()`, `super()`, `import()` —
 * yields undefined: cannot determine, never filter.
 */
export declare function calleeName(call: ts.CallExpression, tsModule: typeof ts): string | undefined;
export declare function callsCarrier(call: ts.CallExpression, fn: ts.FunctionLikeDeclaration, contextualSignatures: ReadonlySet<ts.Node>, checker: ts.TypeChecker, tsModule: typeof ts): boolean;
/**
 * Call-signature declarations of the function type `fn` was written against —
 * its contextual type at the point it is defined (a return-type annotation, a
 * typed variable, a typed property).
 */
export declare function contextualSignatureDeclarations(fn: ts.FunctionLikeDeclaration, checker: ts.TypeChecker, tsModule: typeof ts): Set<ts.Node>;
//# sourceMappingURL=carrier_calls.d.ts.map