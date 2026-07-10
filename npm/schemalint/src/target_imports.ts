import type * as ts from 'typescript';

import { adapterFor, type SdkAdapter } from './sdk_adapters.js';

interface ImportedObject {
  module: string;
  exportPath: string;
}

export interface TargetImports {
  functions: Map<string, SdkAdapter>;
  objects: Map<string, ImportedObject>;
  namespaces: Map<string, string>;
}

export function collectTargetImports(
  sourceFile: ts.SourceFile,
  tsModule: typeof ts
): TargetImports {
  const imports: TargetImports = {
    functions: new Map(),
    objects: new Map(),
    namespaces: new Map(),
  };

  for (const stmt of sourceFile.statements) {
    if (!tsModule.isImportDeclaration(stmt)) continue;
    const spec = stmt.moduleSpecifier;
    const clause = stmt.importClause;
    if (!tsModule.isStringLiteral(spec) || !clause) continue;

    const module = spec.text;
    const bindings = clause.namedBindings;
    if (bindings && tsModule.isNamespaceImport(bindings)) {
      imports.namespaces.set(bindings.name.text, module);
      continue;
    }
    if (!bindings || !tsModule.isNamedImports(bindings)) continue;

    for (const element of bindings.elements) {
      const importedName = element.propertyName?.text ?? element.name.text;
      const localName = element.name.text;
      const adapter = adapterFor(module, importedName);
      if (adapter) {
        imports.functions.set(localName, adapter);
      } else if (hasAdapterPrefix(module, importedName)) {
        imports.objects.set(localName, { module, exportPath: importedName });
      }
    }
  }
  return imports;
}

export function resolveTargetAdapter(
  expression: ts.Expression,
  imports: TargetImports,
  tsModule: typeof ts
): SdkAdapter | undefined {
  if (tsModule.isIdentifier(expression)) {
    return imports.functions.get(expression.text);
  }

  const path = propertyPath(expression, tsModule);
  if (!path || path.length < 2) return undefined;
  const [root, ...members] = path;
  const namespaceModule = imports.namespaces.get(root);
  if (namespaceModule) return adapterFor(namespaceModule, members.join('.'));

  const object = imports.objects.get(root);
  if (!object) return undefined;
  return adapterFor(
    object.module,
    `${object.exportPath}.${members.join('.')}`
  );
}

function hasAdapterPrefix(module: string, exportPath: string): boolean {
  return ['object', 'array'].some((member) =>
    adapterFor(module, `${exportPath}.${member}`)
  );
}

function propertyPath(
  expression: ts.Expression,
  tsModule: typeof ts
): string[] | undefined {
  if (tsModule.isIdentifier(expression)) return [expression.text];
  if (!tsModule.isPropertyAccessExpression(expression)) return undefined;
  const parent = propertyPath(expression.expression, tsModule);
  return parent ? [...parent, expression.name.text] : undefined;
}
