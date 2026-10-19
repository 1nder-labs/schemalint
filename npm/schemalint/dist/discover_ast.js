/**
 * The object literal a Zod object schema is built from: the first argument of
 * the innermost call in its chain (`z.object({...}).extend(...)`,
 * `z.strictObject({...}).describe('x')`). Matched by shape, not by name.
 */
export function findZObjectCall(node, tsModule) {
    let current = node;
    for (;;) {
        while (tsModule.isParenthesizedExpression(current))
            current = current.expression;
        if (!tsModule.isCallExpression(current))
            return null;
        const callee = current.expression;
        if (tsModule.isPropertyAccessExpression(callee) &&
            tsModule.isCallExpression(callee.expression)) {
            current = callee.expression;
            continue;
        }
        const arg = current.arguments[0];
        return arg && tsModule.isObjectLiteralExpression(arg) ? arg : null;
    }
}
/**
 * Escape a single JSON Pointer (RFC 6901) segment.
 *
 * `~` is replaced with `~0` first, then `/` is replaced with `~1`. The order
 * matters: escaping `/` first would introduce new `~1` sequences that the
 * `~` step would then re-escape, corrupting a name that already contains a
 * literal `~1`.
 */
export function escapePointerSegment(segment) {
    return segment.replace(/~/g, '~0').replace(/\//g, '~1');
}
/**
 * Walk an ObjectLiteralExpression (`{ email: z.string(), ... }`) and build a
 * source map mapping JSON Pointer paths to file:line locations.
 *
 * Handles nested `z.object({...})` by recursing into inner object literals.
 */
export function buildSourceMapFromObjectLiteral(objLit, sourceFile, tsModule) {
    const map = {};
    for (const prop of objLit.properties) {
        if (!tsModule.isPropertyAssignment(prop)) {
            // SpreadAssignment and ShorthandPropertyAssignment have no static key
            // that maps to a JSON Pointer — skip them.
            continue;
        }
        // Resolve a static property name.
        // - Identifier:             { email: ... }          → prop.name.text
        // - StringLiteral:          { 'email': ... }        → prop.name.text
        // - ComputedPropertyName with string literal:
        //                           { ['email']: ... }      → inner text
        // - ComputedPropertyName with dynamic expression:
        //                           { [k]: ... }            → skip (not statically known)
        let propName;
        if (tsModule.isIdentifier(prop.name) || tsModule.isStringLiteral(prop.name)) {
            propName = prop.name.text;
        }
        else if (tsModule.isComputedPropertyName(prop.name) &&
            tsModule.isStringLiteral(prop.name.expression)) {
            propName = prop.name.expression.text;
        }
        if (propName === undefined) {
            continue;
        }
        const { line } = sourceFile.getLineAndCharacterOfPosition(prop.getStart(sourceFile));
        // Pointer keys are RFC 6901-escaped: '~' becomes '~0', then '/' becomes
        // '~1'. The Rust normalizer escapes the same way at the same join:
        //   `format!("{}/properties/{}", ptr, escape_pointer_segment(key))`
        //   in crates/schemalint/src/normalize/traverse.rs
        // Both sides must stay byte-identical so that `source_map.get(&pointer)`
        // matches. Escaping only one side would desync the two and break
        // source-mapping for any property whose name contains '/' or '~'.
        const escapedPropName = escapePointerSegment(propName);
        const pointer = `/properties/${escapedPropName}`;
        map[pointer] = {
            file: sourceFile.fileName,
            line: line + 1, // 1-indexed
        };
        // Recurse into nested z.object() values
        const innerCall = findZObjectCall(prop.initializer, tsModule);
        if (innerCall) {
            const nested = buildSourceMapFromObjectLiteral(innerCall, sourceFile, tsModule);
            for (const [nestedPointer, nestedSpan] of Object.entries(nested)) {
                map[`/properties/${escapedPropName}${nestedPointer}`] = nestedSpan;
            }
        }
    }
    return map;
}
export function buildRootSourceMap(node, sourceFile) {
    const { line } = sourceFile.getLineAndCharacterOfPosition(node.getStart(sourceFile));
    return {
        '': {
            file: sourceFile.fileName,
            line: line + 1,
        },
    };
}
//# sourceMappingURL=discover_ast.js.map