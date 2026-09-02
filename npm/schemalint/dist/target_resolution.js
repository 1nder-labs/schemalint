import { stringLiteralText } from './object_properties.js';
import { staticAlternatives, unwrapExpression } from './static_expression.js';
export function namedTarget(api, expression, sourceFile, tsModule, explicitName, metadata) {
    const { line } = sourceFile.getLineAndCharacterOfPosition(expression.getStart(sourceFile));
    const expr = unwrapExpression(expression, tsModule);
    const suffix = explicitName ??
        (tsModule.isIdentifier(expr) ? expr.text : `inline:${line + 1}`);
    return {
        name: `${api}:${suffix}`,
        sourceFile,
        expression,
        metadata,
    };
}
export function spanFor(node, sourceFile) {
    const { line, character } = sourceFile.getLineAndCharacterOfPosition(node.getStart(sourceFile));
    return { file: sourceFile.fileName, line: line + 1, col: character + 1 };
}
export function stringValueFromExpression(expr, checker, tsModule) {
    const values = staticAlternatives(expr, checker, tsModule).map((alternative) => stringLiteralText(alternative, tsModule));
    if (values.length === 0 || values.some((value) => value === undefined)) {
        return undefined;
    }
    const distinct = new Set(values);
    return distinct.size === 1 ? values[0] : undefined;
}
//# sourceMappingURL=target_resolution.js.map