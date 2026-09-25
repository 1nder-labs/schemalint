import { propertyName as propertyNameOf } from './object_properties.js';
import { unwrapExpression } from './static_expression.js';
/**
 * Names a carrier can be invoked under at a call site — the base set before
 * alias expansion. Undefined when no name set is a safe superset: the
 * carrier has no visible binding name (factory return, inline callback), is
 * exported under a caller-chosen name (`export default`, `export =`), or
 * was written against a function type — call sites then invoke it through
 * a parameter or variable of that type under any name at all.
 */
export function carrierBaseNames(fn, contextualSignatures, tsModule) {
    if (contextualSignatures.size > 0)
        return undefined;
    const names = new Set();
    if (fn.name) {
        const own = propertyNameOf(fn.name, tsModule);
        if (own)
            names.add(own);
    }
    const parent = fn.parent;
    if (tsModule.isVariableDeclaration(parent) ||
        tsModule.isBindingElement(parent)) {
        if (tsModule.isIdentifier(parent.name))
            names.add(parent.name.text);
    }
    else if (tsModule.isPropertyAssignment(parent) ||
        tsModule.isPropertyDeclaration(parent) ||
        tsModule.isShorthandPropertyAssignment(parent)) {
        const bound = propertyNameOf(parent.name, tsModule);
        if (bound)
            names.add(bound);
    }
    else if (isAssignment(parent, tsModule)) {
        const bound = assignmentTargetName(parent.left, tsModule);
        if (bound)
            names.add(bound);
    }
    else if (tsModule.isExportAssignment(parent)) {
        return undefined;
    }
    if (names.size === 0 || isDefaultExported(fn, tsModule))
        return undefined;
    return names;
}
/**
 * One syntax-only pass over the selected files collecting carrier
 * invocation aliases. Over-collecting is always safe — a wrong edge only
 * admits a call to the checker, which remains the source of truth.
 */
export function collectInvocationAliases(program, fileSet, tsModule) {
    const edges = new Map();
    const dynamic = new Set();
    function bind(bound, source) {
        if (!bound || !source)
            return;
        const sources = edges.get(bound);
        if (sources) {
            sources.add(source);
        }
        else {
            edges.set(bound, new Set([source]));
        }
    }
    function bindAll(bound, sources) {
        for (const source of sources)
            bind(bound, source);
    }
    function collect(node) {
        if (tsModule.isImportSpecifier(node)) {
            bind(node.name.text, (node.propertyName ?? node.name).text);
        }
        else if (tsModule.isExportSpecifier(node)) {
            const source = (node.propertyName ?? node.name).text;
            if (node.name.text === 'default')
                dynamic.add(source);
            else
                bind(node.name.text, source);
        }
        else if (tsModule.isExportAssignment(node)) {
            const exported = sourceNameOf(node.expression, tsModule);
            if (exported)
                dynamic.add(exported);
        }
        else if (tsModule.isVariableDeclaration(node) ||
            tsModule.isPropertyDeclaration(node)) {
            const bound = bindingName(node.name, tsModule);
            bind(bound, sourceNameOf(node.initializer, tsModule));
            bindAll(bound, typeSourceNames(node.type, tsModule));
            const initializer = node.initializer && unwrapExpression(node.initializer, tsModule);
            if (initializer && tsModule.isArrayLiteralExpression(initializer)) {
                for (const element of initializer.elements) {
                    bind(bound, sourceNameOf(element, tsModule));
                }
            }
        }
        else if (tsModule.isPropertyAssignment(node)) {
            const bound = propertyNameOf(node.name, tsModule);
            bind(bound, sourceNameOf(node.initializer, tsModule));
        }
        else if (tsModule.isPropertySignature(node)) {
            const bound = propertyNameOf(node.name, tsModule);
            bindAll(bound, typeSourceNames(node.type, tsModule));
        }
        else if (tsModule.isBindingElement(node)) {
            if (tsModule.isIdentifier(node.name)) {
                bind(node.name.text, node.propertyName && propertyNameOf(node.propertyName, tsModule));
                bind(node.name.text, sourceNameOf(node.initializer, tsModule));
            }
        }
        else if (tsModule.isTypeAliasDeclaration(node)) {
            bindAll(node.name.text, typeSourceNames(node.type, tsModule));
        }
        else if (isAssignment(node, tsModule)) {
            const source = sourceNameOf(node.right, tsModule);
            if (tsModule.isPropertyAccessExpression(node.left) &&
                tsModule.isIdentifier(node.left.expression) &&
                node.left.expression.text === 'module' &&
                node.left.name.text === 'exports') {
                if (source)
                    dynamic.add(source);
            }
            else {
                bind(assignmentTargetName(node.left, tsModule), source);
            }
        }
    }
    function walk(node) {
        collect(node);
        tsModule.forEachChild(node, walk);
    }
    for (const sourceFile of program.getSourceFiles()) {
        if (sourceFile.isDeclarationFile ||
            sourceFile.fileName.includes('node_modules') ||
            !fileSet.has(sourceFile.fileName)) {
            continue;
        }
        tsModule.forEachChild(sourceFile, walk);
    }
    return { edges, dynamic };
}
/**
 * Chase alias edges to a fixed point. Undefined — the carrier cannot be
 * name-filtered — when a reachable name escapes through a dynamic export
 * (`export default`, `export =`, `module.exports =`) and is therefore
 * invocable under an arbitrary name.
 */
export function expandCarrierNames(base, aliases) {
    const names = new Set(base);
    let grew = true;
    while (grew) {
        grew = false;
        for (const [bound, sources] of aliases.edges) {
            if (names.has(bound))
                continue;
            for (const source of sources) {
                if (names.has(source)) {
                    names.add(bound);
                    grew = true;
                    break;
                }
            }
        }
    }
    for (const name of names) {
        if (aliases.dynamic.has(name))
            return undefined;
    }
    return names;
}
/**
 * The name a call invokes, when statically visible: `f()` → `f`,
 * `a.b.c()` → `c` (covers `this.f`, `super.f`, namespace calls). Any other
 * callee shape — `a[i]()`, `(f)()`, `f()()`, `super()`, `import()` —
 * yields undefined: cannot determine, never filter.
 */
export function calleeName(call, tsModule) {
    const expression = call.expression;
    if (tsModule.isIdentifier(expression))
        return expression.text;
    if (tsModule.isPropertyAccessExpression(expression)) {
        return expression.name.text;
    }
    return undefined;
}
/** `export default function f() {}` puts the default modifier on `fn`. */
function isDefaultExported(fn, tsModule) {
    return (fn.modifiers ?? []).some((modifier) => modifier.kind === tsModule.SyntaxKind.DefaultKeyword);
}
/** `=`, `||=`, `&&=`, `??=` — anything that can leave `target` holding `right`. */
function isAssignment(node, tsModule) {
    if (!tsModule.isBinaryExpression(node))
        return false;
    const kind = node.operatorToken.kind;
    return (kind === tsModule.SyntaxKind.EqualsToken ||
        kind === tsModule.SyntaxKind.BarBarEqualsToken ||
        kind === tsModule.SyntaxKind.AmpersandAmpersandEqualsToken ||
        kind === tsModule.SyntaxKind.QuestionQuestionEqualsToken);
}
/** `f` for `f = x`, `f` for `obj.f = x` (leaf of the access). */
function assignmentTargetName(target, tsModule) {
    if (tsModule.isIdentifier(target))
        return target.text;
    if (tsModule.isPropertyAccessExpression(target))
        return target.name.text;
    return undefined;
}
/** Identifier or string-literal binding/property names; patterns yield nothing. */
function bindingName(name, tsModule) {
    if (!name)
        return undefined;
    if (tsModule.isIdentifier(name) || tsModule.isStringLiteral(name)) {
        return name.text;
    }
    return undefined;
}
/**
 * The name an expression can carry a carrier under: `attempt` for
 * `attempt`, `attempt` for `obj.attempt` (leaf — `const g = obj.attempt`
 * makes `g()` reach the carrier).
 */
function sourceNameOf(expression, tsModule) {
    if (!expression)
        return undefined;
    const unwrapped = unwrapExpression(expression, tsModule);
    if (tsModule.isIdentifier(unwrapped))
        return unwrapped.text;
    if (tsModule.isPropertyAccessExpression(unwrapped)) {
        return unwrapped.name.text;
    }
    return undefined;
}
/**
 * Names a type annotation can carry a carrier under: `x: typeof attempt`
 * binds x to attempt's call signature, `type F = typeof attempt` binds F,
 * and chasing `x: F` through a reference keeps the chain alive.
 */
function typeSourceNames(type, tsModule) {
    if (!type)
        return [];
    if (tsModule.isTypeReferenceNode(type)) {
        return [entityNameLeaf(type.typeName, tsModule)];
    }
    if (tsModule.isTypeQueryNode(type)) {
        return [entityNameLeaf(type.exprName, tsModule)];
    }
    if (tsModule.isUnionTypeNode(type)) {
        return type.types.flatMap((member) => typeSourceNames(member, tsModule));
    }
    if (tsModule.isTypeOperatorNode(type) ||
        tsModule.isParenthesizedTypeNode(type) ||
        tsModule.isArrayTypeNode(type)) {
        const inner = tsModule.isArrayTypeNode(type) ? type.elementType : type.type;
        return typeSourceNames(inner, tsModule);
    }
    return [];
}
/** `A.B.C` → `C`; a bare identifier → its text. */
function entityNameLeaf(name, tsModule) {
    return tsModule.isIdentifier(name) ? name.text : name.right.text;
}
export function callsCarrier(call, fn, contextualSignatures, checker, tsModule) {
    const resolved = checker.getResolvedSignature(call)?.declaration;
    if (resolved) {
        // Direct hit: the callee resolves to the wrapper itself. Signature
        // resolution already follows variables and factory return values.
        if (resolved === fn)
            return true;
        // Indirect hit: the wrapper is called under a function *type* it was
        // written against (`type Compile = ...`), so every call site resolves to
        // that type's signature and the wrapper's own node is never seen.
        if (contextualSignatures.has(resolved)) {
            return true;
        }
    }
    const symbol = checker.getSymbolAtLocation(call.expression);
    const aliased = symbol && (symbol.flags & tsModule.SymbolFlags.Alias)
        ? checker.getAliasedSymbol(symbol)
        : symbol;
    return aliased?.declarations?.some((decl) => decl === fn) ?? false;
}
/**
 * Call-signature declarations of the function type `fn` was written against —
 * its contextual type at the point it is defined (a return-type annotation, a
 * typed variable, a typed property).
 */
export function contextualSignatureDeclarations(fn, checker, tsModule) {
    const declarations = new Set();
    if (!tsModule.isArrowFunction(fn) && !tsModule.isFunctionExpression(fn)) {
        return declarations;
    }
    const contextual = checker.getContextualType(fn);
    if (!contextual)
        return declarations;
    for (const signature of contextual.getCallSignatures()) {
        if (signature.declaration)
            declarations.add(signature.declaration);
    }
    return declarations;
}
//# sourceMappingURL=carrier_calls.js.map