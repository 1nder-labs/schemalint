/**
 * Runtime schema evaluation.
 *
 * Given a file path and export name, dynamically imports the user's TypeScript
 * file, accesses the exported Zod schema, and converts it to JSON Schema.
 *
 * Uses `zod-to-json-schema` by default, but detects Zod v4's native
 * `toJSONSchema()` method when available.
 */
import { randomUUID } from 'node:crypto';
import { existsSync } from 'node:fs';
import { createRequire } from 'node:module';
import { mkdtemp, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { pathToFileURL } from 'node:url';
const localRequire = createRequire(import.meta.url);
/**
 * Dynamically import a user's TypeScript file and evaluate an exported
 * Zod schema to JSON Schema.
 *
 * Requires `tsx` for JIT compilation of TypeScript imports.
 */
export async function evaluateSchema(filePath, exportName) {
    return withStdoutRedirect(async () => {
        const mod = await importModule(filePath);
        const schema = mod[exportName];
        if (!schema) {
            throw new Error(`Export '${exportName}' not found in module '${filePath}'. ` +
                `Available exports: ${Object.keys(mod).join(', ') || '(none)'}`);
        }
        return zodToJsonSchema(schema);
    });
}
export async function evaluateSyntheticSchema(source, exportName, baseFilePath) {
    const dir = await mkdtemp(join(tmpdir(), 'schemalint-zod-'));
    const filePath = join(dir, `${randomUUID()}.ts`);
    try {
        await linkNodeModules(dir, baseFilePath);
        await writeFile(filePath, source, 'utf8');
        return await evaluateSchema(filePath, exportName);
    }
    finally {
        await rm(dir, { recursive: true, force: true });
    }
}
async function withStdoutRedirect(fn) {
    // Redirect stdout to stderr during evaluation so user-code console.log()
    // calls don't corrupt the JSON-RPC protocol channel.
    const originalStdoutWrite = process.stdout.write.bind(process.stdout);
    process.stdout.write = process.stderr.write.bind(process.stderr);
    try {
        return await fn();
    }
    finally {
        // Restore stdout
        process.stdout.write = originalStdoutWrite;
    }
}
async function importModule(filePath) {
    const importPath = filePath.startsWith('file://')
        ? filePath
        : pathToFileURL(filePath).href;
    return (await import(importPath));
}
async function linkNodeModules(dir, baseFilePath) {
    const nodeModules = findNearestNodeModules(baseFilePath);
    if (!nodeModules)
        return;
    try {
        await symlink(nodeModules, join(dir, 'node_modules'), 'dir');
    }
    catch (err) {
        // Bare imports may still be rewritten to file URLs. A symlink failure should
        // not prevent evaluating schemas that do not need package resolution.
        const msg = err instanceof Error ? err.message : String(err);
        console.warn(`[schemalint-zod] Could not symlink node_modules into temp dir: ${msg}`);
    }
}
function findNearestNodeModules(filePath) {
    let dir = dirname(filePath);
    while (true) {
        const candidate = join(dir, 'node_modules');
        if (existsSync(candidate))
            return candidate;
        const parent = dirname(dir);
        if (parent === dir)
            return undefined;
        dir = parent;
    }
}
/**
 * Convert a Zod schema to JSON Schema.
 *
 * Detects Zod v4's native `toJSONSchema()` method and uses it when available.
 * Falls back to `zod-to-json-schema` for Zod v3.
 */
function zodToJsonSchema(schema) {
    // Check for Zod v4 native toJSONSchema()
    if (schema &&
        typeof schema === 'object' &&
        '_def' in schema &&
        'toJSONSchema' in schema &&
        typeof schema.toJSONSchema === 'function') {
        return schema.toJSONSchema();
    }
    // Fall back to zod-to-json-schema (Zod v3)
    // Use createRequire to load from the helper's own node_modules.
    const zodToJsonSchemaModule = localRequire('zod-to-json-schema');
    return zodToJsonSchemaModule.zodToJsonSchema(schema);
}
//# sourceMappingURL=evaluate.js.map