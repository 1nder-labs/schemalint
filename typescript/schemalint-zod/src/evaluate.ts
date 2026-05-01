/**
 * Runtime schema evaluation.
 *
 * Given a file path and export name, dynamically imports the user's TypeScript
 * file, accesses the exported Zod schema, and converts it to JSON Schema.
 *
 * Uses `zod-to-json-schema` by default, but detects Zod v4's native
 * `toJSONSchema()` method when available.
 */

import { createRequire } from 'node:module';

const localRequire = createRequire(import.meta.url);

/**
 * Dynamically import a user's TypeScript file and evaluate an exported
 * Zod schema to JSON Schema.
 *
 * Requires `tsx` for JIT compilation of TypeScript imports.
 */
export async function evaluateSchema(
  filePath: string,
  exportName: string
): Promise<Record<string, unknown>> {
  // Redirect stdout to stderr during evaluation so user-code console.log()
  // calls don't corrupt the JSON-RPC protocol channel.
  const originalStdoutWrite = process.stdout.write.bind(process.stdout);
  process.stdout.write = process.stderr.write.bind(
    process.stderr
  ) as typeof process.stdout.write;

  try {
    // Dynamic import requires a URL
    const importPath = filePath.startsWith('file://')
      ? filePath
      : `file://${filePath}`;

    const mod = await import(importPath);
    const schema = mod[exportName];

    if (!schema) {
      throw new Error(
        `Export '${exportName}' not found in module '${filePath}'. ` +
          `Available exports: ${Object.keys(mod).join(', ') || '(none)'}`
      );
    }

    return zodToJsonSchema(schema);
  } finally {
    // Restore stdout
    process.stdout.write = originalStdoutWrite;
  }
}

/**
 * Convert a Zod schema to JSON Schema.
 *
 * Detects Zod v4's native `toJSONSchema()` method and uses it when available.
 * Falls back to `zod-to-json-schema` for Zod v3.
 */
function zodToJsonSchema(schema: unknown): Record<string, unknown> {
  // Check for Zod v4 native toJSONSchema()
  if (
    schema &&
    typeof schema === 'object' &&
    '_def' in schema &&
    'toJSONSchema' in schema &&
    typeof (schema as Record<string, unknown>).toJSONSchema === 'function'
  ) {
    return (
      schema as { toJSONSchema: () => Record<string, unknown> }
    ).toJSONSchema();
  }

  // Fall back to zod-to-json-schema (Zod v3)
  // Use createRequire to load from the helper's own node_modules.
  const zodToJsonSchemaModule = localRequire('zod-to-json-schema') as {
    zodToJsonSchema: (s: unknown) => unknown;
  };
  return zodToJsonSchemaModule.zodToJsonSchema(schema) as Record<
    string,
    unknown
  >;
}
