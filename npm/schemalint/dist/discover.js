import { realpathSync } from 'node:fs';
import path from 'node:path';
import { evaluateSyntheticSchema } from './evaluate.js';
/**
 * Normalize a file-system path to forward slashes.
 *
 * `path.relative()` returns backslash-separated paths on Windows
 * (e.g. `src\\foo.ts`), but picomatch globs use forward slashes.
 * Replacing the OS separator with `/` is a no-op on POSIX and correct
 * on Windows, making the glob filter work on both platforms.
 *
 * The `sep` parameter exists solely for unit-testing Windows paths on a
 * POSIX machine — pass `'\\'` to simulate Windows `path.sep`.
 */
export function toPosixPath(p, sep = path.sep) {
    return sep === '/' ? p : p.split(sep).join('/');
}
import { exportedSchemaTargets } from './exported_targets.js';
import { groupByProject, listScopeFiles, } from './projects.js';
import { resolveSourceScope } from './source_scope.js';
import { findSchemaTargets } from './targets.js';
/**
 * Name the cause of an empty discovery result: the source scope matched no
 * TypeScript file on disk.
 */
function emptyDiscoveryWarning(sourceGlob) {
    return {
        model: '',
        message: `No file on disk matched source glob '${sourceGlob}'.`,
    };
}
/** Import each target and convert it to JSON Schema, recording per-target failures. */
async function evaluateTargets(locations, failures) {
    const models = [];
    for (const loc of locations) {
        try {
            const schemaJson = await evaluateSyntheticSchema(loc.syntheticSource, loc.exportName, loc.filePath);
            models.push({
                name: loc.name,
                module_path: loc.filePath,
                schema: schemaJson,
                source_map: loc.sourceMap,
                canonical_kind: loc.canonicalKind,
                provider: loc.provider,
                envelope: loc.envelope,
                usage_span: loc.usageSpan,
            });
        }
        catch (err) {
            const message = err instanceof Error ? err.message : String(err);
            failures.push({
                kind: 'evaluation',
                target: loc.name,
                message: `Failed to evaluate schema '${loc.name}' in ${loc.filePath}: ${message}`,
            });
        }
    }
    return models;
}
/** Build one program for a tsconfig group and discover its schemas. */
async function discoverInProject(group, scope, tsModule, acc) {
    const program = tsModule.createProgram(group.files, group.compilerOptions);
    const fileSet = new Set(group.files);
    const selectedSourceFiles = program.getSourceFiles().filter((sourceFile) => !sourceFile.isDeclarationFile &&
        !sourceFile.fileName.includes('node_modules') &&
        fileSet.has(sourceFile.fileName));
    // Step 1: schemas traced to a provider call site — AI SDK, OpenAI helpers,
    // Anthropic helpers — through any number of wrapper functions.
    const callsiteDiscovery = findSchemaTargets(program, fileSet, tsModule, group.compilerOptions);
    const traced = callsiteDiscovery.targets.filter((target) => !target.declaration || !acc.reported.has(target.declaration));
    for (const target of traced) {
        if (target.declaration)
            acc.reported.add(target.declaration);
    }
    acc.locations += traced.length;
    acc.discoveryFailures += callsiteDiscovery.failures.length;
    acc.failures.push(...callsiteDiscovery.failures);
    acc.models.push(...(await evaluateTargets(traced, acc.failures)));
    // Step 2: exported schemas. In explicit scope (the source named a file or
    // directory) the user pointed at these files, so every exported schema in
    // them is linted alongside the traced ones. In glob scope only traced
    // schemas count, and exported schemas are a fallback solely for projects
    // that use no known provider SDK at all: when the SDK is present but
    // nothing traced, linting every export would flag schemas that never reach
    // a model, so the run reports the gap instead.
    if (scope.explicit || callsiteDiscovery.sdkFiles === 0) {
        const tracedDeclarations = new Set(callsiteDiscovery.targets.map((target) => target.declaration));
        const exported = exportedSchemaTargets(selectedSourceFiles, program.getTypeChecker(), tsModule, group.compilerOptions).filter((target) => !tracedDeclarations.has(target.declaration) &&
            !acc.reported.has(target.declaration ?? ''));
        for (const target of exported)
            acc.reported.add(target.declaration ?? '');
        acc.locations += exported.length;
        acc.models.push(...(await evaluateTargets(exported, acc.failures)));
    }
    else {
        acc.sdkFiles += callsiteDiscovery.sdkFiles;
    }
}
/**
 * Discover Zod schemas by walking TypeScript ASTs.
 *
 * 1. Lists the TypeScript files the source scope names, from disk.
 * 2. Groups them by nearest tsconfig.json and builds one program per group;
 *    files with no tsconfig above them share a default program.
 * 3. Walks each program's ASTs looking for schemas reaching a provider call
 *    or exported `z.object({...})` calls, following re-exports and aliases.
 * 4. Extracts property source locations for source map.
 * 5. Dynamically imports each schema and evaluates it at runtime.
 * 6. Converts schemas to JSON Schema via zod-to-json-schema or native.
 */
export async function discoverZodSchemas(sourceGlob, exclusions = []) {
    const tsModule = await import('typescript');
    const pm = await import('picomatch');
    const picomatch = typeof pm.default === 'function' ? pm.default : pm;
    if (typeof picomatch !== 'function') {
        throw new Error('Failed to load picomatch: expected a function but got ' +
            typeof picomatch +
            '. Check that picomatch is correctly installed.');
    }
    // Real path on both sides: files are listed and compared as real paths, so
    // a symlinked working directory (macOS /tmp) still matches its own files.
    const projectRoot = realpathSync(process.cwd());
    const scope = resolveSourceScope(sourceGlob, picomatch, projectRoot);
    let fileNames = listScopeFiles(scope, projectRoot, tsModule, picomatch);
    const matchedFiles = fileNames.length;
    const excludeMatchers = exclusions.map((pattern) => picomatch(pattern, { dot: true }));
    fileNames = fileNames.filter((file) => {
        const relative = toPosixPath(path.relative(projectRoot, file));
        return !excludeMatchers.some((matches) => matches(relative));
    });
    const excluded = matchedFiles - fileNames.length;
    if (fileNames.length === 0) {
        // Only diagnose a cause when the scope itself matched nothing. When
        // files matched but were then all removed by --exclude, that is ordinary
        // exclusion behavior, not a discovery problem.
        return {
            models: [],
            warnings: matchedFiles === 0 ? [emptyDiscoveryWarning(scope.pattern)] : [],
            failures: [],
            counts: { attempted: 0, excluded, discovered: 0, failed: 0 },
        };
    }
    const acc = {
        models: [],
        failures: [],
        discoveryFailures: 0,
        locations: 0,
        sdkFiles: 0,
        reported: new Set(),
    };
    // A named file bypasses tsconfig include/exclude; a directory or glob does not.
    const groups = groupByProject(fileNames, tsModule, scope.target?.kind !== 'file');
    if (groups.length === 0) {
        return {
            models: [],
            warnings: [
                {
                    model: '',
                    message: `${fileNames.length} file(s) matched source glob '${sourceGlob}' ` +
                        'but all are excluded by tsconfig.json. Name a file directly to check it.',
                },
            ],
            failures: [],
            counts: { attempted: 0, excluded, discovered: 0, failed: 0 },
        };
    }
    for (const group of groups) {
        await discoverInProject(group, scope, tsModule, acc);
    }
    const warnings = [];
    if (acc.locations === 0 && acc.sdkFiles > 0) {
        // Traced-but-failed targets already surface as failures; this names the
        // case where tracing itself found nothing.
        warnings.push({
            model: '',
            message: `${acc.sdkFiles} file(s) matched by source glob ` +
                `'${sourceGlob}' import a provider SDK, but no schema could be ` +
                'traced to a provider call site, so nothing was checked. Name a ' +
                'file or directory instead of a glob to lint every exported schema in it.',
        });
    }
    if (acc.locations === 0 && acc.discoveryFailures === 0) {
        return {
            models: [],
            warnings: warnings.length > 0
                ? warnings
                : [
                    {
                        model: '',
                        message: `Checked ${fileNames.length} file(s) matched by source glob ` +
                            `'${sourceGlob}' for Zod schemas but found none.`,
                    },
                ],
            failures: [],
            counts: { attempted: 0, excluded, discovered: 0, failed: 0 },
        };
    }
    return {
        models: acc.models,
        warnings,
        failures: acc.failures,
        counts: {
            attempted: acc.locations + acc.discoveryFailures,
            excluded,
            discovered: acc.models.length,
            failed: acc.failures.length,
        },
    };
}
//# sourceMappingURL=discover.js.map