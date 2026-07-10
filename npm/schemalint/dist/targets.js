import { resolveTarget } from './target_emit.js';
import { collectTargetImports, resolveTargetAdapter, } from './target_imports.js';
import { collectCarrierTargets, propertyFromExpression, pushExpressionOrCarrier, spanFor, stringValueFromExpression, } from './target_resolution.js';
import { unambiguousExpression } from './static_expression.js';
export function findSchemaTargets(program, fileSet, tsModule, compilerOptions) {
    const checker = program.getTypeChecker();
    const carrierExpressions = [];
    const targets = [];
    const failures = [];
    const seen = new Set();
    for (const sourceFile of program.getSourceFiles()) {
        if (sourceFile.isDeclarationFile ||
            sourceFile.fileName.includes('node_modules') ||
            !fileSet.has(sourceFile.fileName)) {
            continue;
        }
        const found = collectTargetExpressions(sourceFile, checker, tsModule, carrierExpressions, failures);
        for (const target of found) {
            pushTarget(targets, seen, resolveTarget(target, checker, tsModule, compilerOptions));
        }
    }
    for (const target of collectCarrierTargets(program, fileSet, checker, tsModule, carrierExpressions)) {
        pushTarget(targets, seen, resolveTarget(target, checker, tsModule, compilerOptions));
    }
    return { targets, failures };
}
function collectTargetExpressions(sourceFile, checker, tsModule, carrierExpressions, failures) {
    const targets = [];
    const imports = collectTargetImports(sourceFile, tsModule);
    function walk(node) {
        if (tsModule.isCallExpression(node)) {
            collectFromCall(node, sourceFile, checker, tsModule, imports, targets, carrierExpressions, failures);
        }
        tsModule.forEachChild(node, walk);
    }
    tsModule.forEachChild(sourceFile, walk);
    return targets;
}
function collectFromCall(call, sourceFile, checker, tsModule, imports, targets, carriers, failures) {
    const adapter = resolveTargetAdapter(call.expression, imports, tsModule);
    if (!adapter)
        return;
    const schema = schemaExpression(call, adapter, checker, tsModule);
    if (!schema) {
        failures.push({
            kind: 'metadata',
            target: adapter.kind,
            message: `required schema metadata is not statically resolvable at ${formatSpan(spanFor(call, sourceFile))}`,
        });
        return;
    }
    const envelope = {};
    for (const selector of adapter.envelope) {
        const expression = selector.property
            ? propertyFromExpression(call.arguments[selector.argument], selector.property, checker, tsModule)
            : call.arguments[selector.argument];
        const value = stringValueFromExpression(expression, checker, tsModule);
        if (selector.required && value === undefined) {
            failures.push({
                kind: 'metadata',
                target: adapter.kind,
                message: `required field '${selector.name}' is not statically resolvable at ${formatSpan(spanFor(expression ?? call, sourceFile))}`,
            });
            return;
        }
        if (expression) {
            envelope[selector.name] = {
                required: selector.required,
                span: spanFor(expression, sourceFile),
                ...(value === undefined ? {} : { value }),
            };
        }
    }
    const metadata = {
        adapterModule: adapter.module,
        canonicalKind: adapter.kind,
        provider: adapter.provider
            ? { certainty: 'definitive', provider: adapter.provider }
            : { certainty: 'ambiguous' },
        envelope,
        usageSpan: spanFor(call, sourceFile),
    };
    pushExpressionOrCarrier(targets, carriers, adapter.exportPath, schema, sourceFile, tsModule, envelope.name?.value, metadata);
}
function schemaExpression(call, adapter, checker, tsModule) {
    if ('properties' in adapter.schema) {
        for (const property of adapter.schema.properties) {
            const found = propertyFromExpression(call.arguments[adapter.schema.argument], property, checker, tsModule);
            const stable = unambiguousExpression(found, checker, tsModule);
            if (stable)
                return stable;
        }
        return undefined;
    }
    return unambiguousExpression(call.arguments[adapter.schema.argument], checker, tsModule);
}
function pushTarget(targets, seen, resolved) {
    const usage = resolved.usageSpan;
    const key = `${resolved.canonicalKind}:${usage.file}:${usage.line}:${usage.col}:${resolved.name}`;
    if (seen.has(key))
        return;
    seen.add(key);
    targets.push(resolved);
}
function formatSpan(span) {
    return `${span.file}:${span.line}:${span.col}`;
}
//# sourceMappingURL=targets.js.map