import { buildRootSourceMap, buildSourceMapFromObjectLiteral, findZObjectCall, } from './discover_ast.js';
/**
 * Every Zod object schema `sourceFiles` can build without running a
 * function — declared or inline, exported or not, at module scope or inside a
 * function body — plus the schemas they export, followed through re-exports
 * and aliases to where each is built.
 *
 * No SDK, wrapper, or spelling is tracked: whatever a schema is later handed
 * to, its value is built here. A schema that uses a parameter, a local, or
 * `this` depends on run-time input, so it cannot be checked statically and is
 * skipped, though a buildable schema inside it is still found. A schema nested
 * in a buildable one is linted as part of its root.
 */
export function findSchemaRoots(sourceFiles, checker, tsModule) {
    const roots = new Map();
    const add = (root) => {
        if (!roots.has(root.key))
            roots.set(root.key, root);
    };
    const mayBuild = zodBuilderCheck(checker, tsModule);
    for (const sourceFile of sourceFiles) {
        const visit = (node) => {
            if (tsModule.isCallExpression(node) &&
                mayBuild(node) &&
                isZodObject(node, checker)) {
                if (buildable(node, checker, tsModule)) {
                    add(rootOf(node, sourceFile, tsModule));
                    return;
                }
            }
            tsModule.forEachChild(node, visit);
        };
        tsModule.forEachChild(sourceFile, visit);
        // A block-scoped `var` can be exported but not sliced; the walk above
        // already found it as an expression.
        for (const decl of exportedSchemas(sourceFile, checker, tsModule)) {
            if (topLevel(decl, tsModule))
                add(declared(decl, tsModule));
        }
    }
    return [...roots.values()];
}
// ponytail: duck-typed on Zod's own members — `_zod` (v4) or `_def` (v3), plus
// `shape`, which only object schemas have — so no import or spelling is tracked.
function isZodObject(node, checker) {
    const type = checker.getTypeAtLocation(node);
    const has = (name) => checker.getPropertyOfType(type, name) !== undefined;
    return has('shape') && (has('_zod') || has('_def'));
}
function isZodValue(type, checker) {
    return (checker.getPropertyOfType(type, '_zod') !== undefined ||
        checker.getPropertyOfType(type, '_def') !== undefined);
}
const ZOD_PACKAGE = /[\\/]node_modules[\\/]zod[\\/]/;
/**
 * Whether a call can build a Zod schema, judged by the leftmost name in its
 * chain (`z` in `z.object(...)`, `Base` in `Base.extend(...)`, `make` in
 * `make()`): something from the zod package, a Zod schema, or a function
 * returning one. Decided once per name, so an app's every `db.select()...`
 * or `useQuery(...)` call never pays for full type inference.
 */
function zodBuilderCheck(checker, tsModule) {
    const verdicts = new Map();
    const importsZod = new Map();
    // Only code that imports zod can declare a schema or a schema factory, so
    // only names declared there are worth typing — typing every name forces
    // inference of every component and query-builder body in the app.
    const declaredBesideZod = (file) => {
        let found = importsZod.get(file);
        if (found === undefined) {
            found = file.statements.some((stmt) => (tsModule.isImportDeclaration(stmt) || tsModule.isExportDeclaration(stmt)) &&
                stmt.moduleSpecifier !== undefined &&
                tsModule.isStringLiteral(stmt.moduleSpecifier) &&
                /^zod(\/|$)/.test(stmt.moduleSpecifier.text));
            importsZod.set(file, found);
        }
        return found;
    };
    const decide = (symbol, at) => {
        const target = symbol.flags & tsModule.SymbolFlags.Alias ? checker.getAliasedSymbol(symbol) : symbol;
        const declarations = target.declarations ?? [];
        const files = declarations.map((decl) => decl.getSourceFile());
        if (files.some((file) => ZOD_PACKAGE.test(file.fileName)))
            return true;
        // A value derived from an imported schema (`Middle = Base.pick(...)`) is
        // declared where zod is never imported: follow its initializer instead.
        const derived = declarations.some((decl) => tsModule.isVariableDeclaration(decl) &&
            decl.initializer !== undefined &&
            headCanBuild(decl.initializer));
        if (!derived && !files.some(declaredBesideZod))
            return false;
        const type = checker.getTypeOfSymbolAtLocation(symbol, at);
        return (isZodValue(type, checker) ||
            type
                .getCallSignatures()
                .some((signature) => isZodValue(signature.getReturnType(), checker)));
    };
    function headCanBuild(expression) {
        let head = expression;
        for (;;) {
            if (tsModule.isPropertyAccessExpression(head) ||
                tsModule.isCallExpression(head) ||
                tsModule.isParenthesizedExpression(head) ||
                tsModule.isNonNullExpression(head) ||
                tsModule.isAsExpression(head) ||
                tsModule.isSatisfiesExpression(head)) {
                head = head.expression;
            }
            else {
                break;
            }
        }
        if (!tsModule.isIdentifier(head))
            return false;
        const symbol = checker.getSymbolAtLocation(head);
        if (!symbol)
            return false;
        let verdict = verdicts.get(symbol);
        if (verdict === undefined) {
            verdicts.set(symbol, false); // a cycle of aliases builds nothing
            verdict = decide(symbol, head);
            verdicts.set(symbol, verdict);
        }
        return verdict;
    }
    return (call) => headCanBuild(call.expression);
}
/** Every name `root` uses is imported, declared at module level, or declared inside `root`. */
function buildable(root, checker, tsModule) {
    let ok = true;
    const visit = (node) => {
        if (!ok || tsModule.isTypeNode(node))
            return;
        if (node.kind === tsModule.SyntaxKind.ThisKeyword) {
            ok = false;
            return;
        }
        if (tsModule.isIdentifier(node) && !isMemberName(node, tsModule)) {
            const symbol = tsModule.isShorthandPropertyAssignment(node.parent)
                ? checker.getShorthandAssignmentValueSymbol(node.parent)
                : checker.getSymbolAtLocation(node);
            const decl = symbol?.declarations?.[0];
            if (decl && !contains(root, decl) && !topLevel(decl, tsModule))
                ok = false;
        }
        tsModule.forEachChild(node, visit);
    };
    visit(root);
    return ok;
}
function isMemberName(id, tsModule) {
    const parent = id.parent;
    return ((tsModule.isPropertyAccessExpression(parent) && parent.name === id) ||
        (tsModule.isPropertyAssignment(parent) && parent.name === id));
}
function contains(root, node) {
    return (node.getSourceFile() === root.getSourceFile() &&
        node.pos >= root.pos &&
        node.end <= root.end);
}
/**
 * Declared where the sliced module can reach it: an import, a top-level
 * statement, or an ambient declaration. Anything in a function, class, block,
 * or loop exists only at run time (and is not kept by the slicer).
 */
function topLevel(node, tsModule) {
    if (node.getSourceFile().isDeclarationFile)
        return true;
    for (let current = node.parent; current && !tsModule.isSourceFile(current); current = current.parent) {
        if (tsModule.isBlock(current) ||
            tsModule.isModuleBlock(current) ||
            tsModule.isCaseBlock(current) ||
            tsModule.isCatchClause(current) ||
            tsModule.isIterationStatement(current, false) ||
            tsModule.isFunctionLike(current) ||
            tsModule.isClassLike(current)) {
            return false;
        }
    }
    return true;
}
function rootOf(call, sourceFile, tsModule) {
    let outer = call;
    while (isOuterExpression(outer.parent, tsModule))
        outer = outer.parent;
    const parent = outer.parent;
    const declaration = tsModule.isVariableDeclaration(parent) &&
        parent.initializer === outer &&
        tsModule.isIdentifier(parent.name)
        ? parent
        : undefined;
    // A module-level name seeds the slice by its declaration, so the schema is
    // reported once however many files reach it.
    if (declaration && topLevel(declaration, tsModule)) {
        return declared(declaration, tsModule);
    }
    const { line } = sourceFile.getLineAndCharacterOfPosition(call.getStart(sourceFile));
    const name = declaration
        ? declaration.name.text
        : tsModule.isExportAssignment(parent)
            ? 'default'
            : `inline:${line + 1}`;
    return {
        name,
        sourceFile,
        seed: call,
        sourceMap: sourceMapOf(call, sourceFile, tsModule),
        key: `${sourceFile.fileName}@${call.getStart(sourceFile)}`,
    };
}
function declared(decl, tsModule) {
    const name = decl.name;
    const sourceFile = decl.getSourceFile();
    return {
        name: name.text,
        sourceFile,
        seed: name,
        sourceMap: sourceMapOf(decl.initializer ?? name, sourceFile, tsModule),
        key: `${sourceFile.fileName}#${name.text}`,
    };
}
/**
 * The declarations behind a file's exported schemas, followed through
 * re-exports (`export { X } from`) and value aliases (`export const B = A`)
 * to where each schema is built, so an alias is reported once, as its source.
 */
function exportedSchemas(sourceFile, checker, tsModule) {
    const moduleSymbol = checker.getSymbolAtLocation(sourceFile);
    if (!moduleSymbol)
        return [];
    const resolve = (symbol) => {
        const seen = new Set();
        while (!seen.has(symbol)) {
            seen.add(symbol);
            const target = symbol.flags & tsModule.SymbolFlags.Alias ? checker.getAliasedSymbol(symbol) : symbol;
            const decl = target.valueDeclaration;
            if (!decl ||
                !tsModule.isVariableDeclaration(decl) ||
                !tsModule.isIdentifier(decl.name) ||
                !decl.initializer) {
                return undefined;
            }
            let initializer = decl.initializer;
            while (isOuterExpression(initializer, tsModule))
                initializer = initializer.expression;
            if (!tsModule.isIdentifier(initializer))
                return decl;
            const next = checker.getSymbolAtLocation(initializer);
            if (!next)
                return undefined;
            symbol = next;
        }
        return undefined;
    };
    return checker.getExportsOfModule(moduleSymbol).flatMap((exported) => {
        const decl = resolve(exported);
        const file = decl?.getSourceFile();
        return decl &&
            file &&
            !file.isDeclarationFile &&
            !file.fileName.includes('node_modules') &&
            isZodObject(decl.name, checker)
            ? [decl]
            : [];
    });
}
/** A wrapper that leaves an expression's value as it is: `( )`, `as`, `satisfies`, `!`. */
function isOuterExpression(node, tsModule) {
    return (tsModule.isParenthesizedExpression(node) ||
        tsModule.isAsExpression(node) ||
        tsModule.isSatisfiesExpression(node) ||
        tsModule.isNonNullExpression(node));
}
function sourceMapOf(expr, sourceFile, tsModule) {
    const objectArg = findZObjectCall(expr, tsModule);
    return {
        ...buildRootSourceMap(expr, sourceFile),
        ...(objectArg ? buildSourceMapFromObjectLiteral(objectArg, sourceFile, tsModule) : {}),
    };
}
//# sourceMappingURL=schema_roots.js.map