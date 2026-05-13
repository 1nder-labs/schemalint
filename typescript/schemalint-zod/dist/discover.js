import { evaluateSchema } from './evaluate.js';
import { buildSourceMapFromObjectLiteral, findExportedSchemaCalls, scanProviderImports, scanZodTextFormatRefs, } from './discover_ast.js';
/**
 * Discover Zod schemas by walking TypeScript ASTs.
 *
 * 1. Reads tsconfig.json to resolve the project file list.
 * 2. Filters files against the user-supplied source glob.
 * 3. Walks each source file's AST looking for `z.object({...})` calls.
 * 4. Extracts property source locations for source map.
 * 5. Dynamically imports each file and evaluates schemas at runtime.
 * 6. Converts schemas to JSON Schema via zod-to-json-schema or native.
 */
export async function discoverZodSchemas(sourceGlob) {
    const tsModule = await import('typescript');
    const pm = await import('picomatch');
    // Resolve tsconfig.json
    const configPath = tsModule.findConfigFile(process.cwd(), tsModule.sys.fileExists, 'tsconfig.json');
    if (!configPath) {
        throw new Error('No tsconfig.json found in the current project. ' +
            'Run this tool from a TypeScript project root.');
    }
    const configFile = tsModule.readConfigFile(configPath, tsModule.sys.readFile);
    if (configFile.error) {
        throw new Error(`Failed to read tsconfig.json: ${tsModule.flattenDiagnosticMessageText(configFile.error.messageText, '\n')}`);
    }
    const parsedConfig = tsModule.parseJsonConfigFileContent(configFile.config, tsModule.sys, process.cwd());
    const compilerOptions = parsedConfig.options;
    // Resolve the full file list from tsconfig
    let fileNames = parsedConfig.fileNames;
    if (fileNames.length === 0) {
        throw new Error('No source files found in tsconfig.json. Ensure "include" patterns match your project files.');
    }
    // Filter files against the source glob.
    // tsconfig resolves absolute paths, but glob patterns match relative paths.
    // Convert each file to a path relative to the project root before matching.
    const isMatch = pm.default(sourceGlob, { dot: true });
    const projectRoot = process.cwd();
    fileNames = fileNames.filter((f) => {
        const relPath = f.startsWith(projectRoot)
            ? f.slice(projectRoot.length + 1)
            : f;
        return isMatch(relPath);
    });
    if (fileNames.length === 0) {
        return { models: [], warnings: [] };
    }
    // Create program and walk ASTs to discover schemas
    const program = tsModule.createProgram(fileNames, compilerOptions);
    // Step 0: Scan provider imports for auto-detection
    let providerHint;
    for (const sourceFile of program.getSourceFiles()) {
        if (sourceFile.isDeclarationFile || sourceFile.fileName.includes('node_modules'))
            continue;
        if (!fileNames.includes(sourceFile.fileName))
            continue;
        const hint = scanProviderImports(sourceFile, tsModule);
        if (hint) {
            providerHint = hint;
            break;
        }
    }
    // Step 1: Collect zodTextFormat / zodResponseFormat references
    // These point to schemas that should be discovered even if not top-level exported.
    const zodFormatRefs = [];
    for (const sourceFile of program.getSourceFiles()) {
        if (sourceFile.isDeclarationFile || sourceFile.fileName.includes('node_modules'))
            continue;
        if (!fileNames.includes(sourceFile.fileName))
            continue;
        for (const ref of scanZodTextFormatRefs(sourceFile, tsModule)) {
            zodFormatRefs.push(ref);
        }
    }
    // Step 2: AST walk to find z.object() calls and extract source locations
    const discoveredLocations = [];
    const nonFatal = [];
    const zodFormatRefSet = new Set(zodFormatRefs);
    for (const sourceFile of program.getSourceFiles()) {
        // Skip lib files (e.g., node_modules, TypeScript DOM libs)
        if (sourceFile.isDeclarationFile ||
            sourceFile.fileName.includes('node_modules')) {
            continue;
        }
        // Only walk files in our filtered list
        if (!fileNames.includes(sourceFile.fileName))
            continue;
        const exports = findExportedSchemaCalls(sourceFile, tsModule, zodFormatRefSet);
        for (const exp of exports) {
            const sourceMap = buildSourceMapFromObjectLiteral(exp.objectArg, sourceFile, tsModule);
            discoveredLocations.push({
                name: exp.name,
                filePath: sourceFile.fileName,
                sourceMap,
            });
        }
    }
    if (discoveredLocations.length === 0) {
        return { models: [], warnings: nonFatal };
    }
    // Step 3: Runtime evaluation — import each file and evaluate schemas
    const models = [];
    for (const loc of discoveredLocations) {
        try {
            const schemaJson = await evaluateSchema(loc.filePath, loc.name);
            models.push({
                name: loc.name,
                module_path: loc.filePath,
                schema: schemaJson,
                source_map: loc.sourceMap,
            });
        }
        catch (err) {
            const message = err instanceof Error ? err.message : String(err);
            nonFatal.push({
                model: loc.name,
                message: `Failed to evaluate schema '${loc.name}' in ${loc.filePath}: ${message}`,
            });
        }
    }
    const response = { models, warnings: nonFatal };
    if (providerHint) {
        response.provider_hint = providerHint;
    }
    return response;
}
//# sourceMappingURL=discover.js.map