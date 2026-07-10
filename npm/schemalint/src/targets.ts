import type * as ts from 'typescript';

import { resolveTarget, type SchemaTarget } from './target_emit.js';
import {
  collectTargetImports,
  resolveTargetAdapter,
  type TargetImports,
} from './target_imports.js';
import {
  collectCarrierTargets,
  propertyFromExpression,
  pushExpressionOrCarrier,
  spanFor,
  stringValueFromExpression,
  type CarrierExpression,
  type TargetExpression,
  type TargetMetadata,
} from './target_resolution.js';
import type { EnvelopeField, SdkAdapter } from './sdk_adapters.js';

export type { SchemaTarget } from './target_emit.js';

export interface TargetFailure {
  kind: 'metadata';
  target: string;
  message: string;
}

export interface TargetDiscovery {
  targets: SchemaTarget[];
  failures: TargetFailure[];
}

export function findSchemaTargets(
  program: ts.Program,
  fileSet: ReadonlySet<string>,
  tsModule: typeof ts,
  compilerOptions: ts.CompilerOptions
): TargetDiscovery {
  const checker = program.getTypeChecker();
  const carrierExpressions: CarrierExpression[] = [];
  const targets: SchemaTarget[] = [];
  const failures: TargetFailure[] = [];
  const seen = new Set<string>();

  for (const sourceFile of program.getSourceFiles()) {
    if (
      sourceFile.isDeclarationFile ||
      sourceFile.fileName.includes('node_modules') ||
      !fileSet.has(sourceFile.fileName)
    ) {
      continue;
    }

    const found = collectTargetExpressions(
      sourceFile,
      checker,
      tsModule,
      carrierExpressions,
      failures
    );
    for (const target of found) {
      pushTarget(
        targets,
        seen,
        resolveTarget(target, checker, tsModule, compilerOptions)
      );
    }
  }

  for (const target of collectCarrierTargets(
    program,
    fileSet,
    checker,
    tsModule,
    carrierExpressions
  )) {
    pushTarget(
      targets,
      seen,
      resolveTarget(target, checker, tsModule, compilerOptions)
    );
  }

  inferSingleProvider(targets);
  return { targets, failures };
}

function collectTargetExpressions(
  sourceFile: ts.SourceFile,
  checker: ts.TypeChecker,
  tsModule: typeof ts,
  carrierExpressions: CarrierExpression[],
  failures: TargetFailure[]
): TargetExpression[] {
  const targets: TargetExpression[] = [];
  const imports = collectTargetImports(sourceFile, tsModule);

  function walk(node: ts.Node): void {
    if (tsModule.isCallExpression(node)) {
      collectFromCall(
        node,
        sourceFile,
        checker,
        tsModule,
        imports,
        targets,
        carrierExpressions,
        failures
      );
    }
    tsModule.forEachChild(node, walk);
  }

  tsModule.forEachChild(sourceFile, walk);
  return targets;
}

function collectFromCall(
  call: ts.CallExpression,
  sourceFile: ts.SourceFile,
  checker: ts.TypeChecker,
  tsModule: typeof ts,
  imports: TargetImports,
  targets: TargetExpression[],
  carriers: CarrierExpression[],
  failures: TargetFailure[]
): void {
  const adapter = resolveTargetAdapter(call.expression, imports, tsModule);
  if (!adapter) return;

  const schema = schemaExpression(call, adapter, checker, tsModule);
  if (!schema) {
    failures.push({
      kind: 'metadata',
      target: adapter.kind,
      message: `required schema metadata is not statically resolvable at ${formatSpan(
        spanFor(call, sourceFile)
      )}`,
    });
    return;
  }

  const envelope: Record<string, EnvelopeField> = {};
  for (const selector of adapter.envelope) {
    const expression = selector.property
      ? propertyFromExpression(
          call.arguments[selector.argument],
          selector.property,
          checker,
          tsModule
        )
      : call.arguments[selector.argument];
    const value = stringValueFromExpression(expression, checker, tsModule);
    if (selector.required && value === undefined) {
      failures.push({
        kind: 'metadata',
        target: adapter.kind,
        message: `required field '${selector.name}' is not statically resolvable at ${formatSpan(
          spanFor(expression ?? call, sourceFile)
        )}`,
      });
      return;
    }
    if (expression) {
      envelope[selector.name] = {
        required: selector.required,
        resolved: value !== undefined,
        span: spanFor(expression, sourceFile),
        ...(value === undefined ? {} : { value }),
      };
    }
  }

  const metadata: TargetMetadata = {
    adapterModule: adapter.module,
    canonicalKind: adapter.kind,
    provider: adapter.provider
      ? { certainty: 'definitive', provider: adapter.provider }
      : { certainty: 'ambiguous' },
    envelope,
    usageSpan: spanFor(call, sourceFile),
  };
  pushExpressionOrCarrier(
    targets,
    carriers,
    adapter.exportPath,
    schema,
    sourceFile,
    tsModule,
    envelope.name?.value,
    metadata
  );
}

function schemaExpression(
  call: ts.CallExpression,
  adapter: SdkAdapter,
  checker: ts.TypeChecker,
  tsModule: typeof ts
): ts.Expression | undefined {
  if ('properties' in adapter.schema) {
    for (const property of adapter.schema.properties) {
      const found = propertyFromExpression(
        call.arguments[adapter.schema.argument],
        property,
        checker,
        tsModule
      );
      if (found) return found;
    }
    return undefined;
  }
  return call.arguments[adapter.schema.argument];
}

function pushTarget(
  targets: SchemaTarget[],
  seen: Set<string>,
  resolved: SchemaTarget
): void {
  const usage = resolved.usageSpan;
  const key = `${resolved.canonicalKind}:${usage.file}:${usage.line}:${usage.col}:${resolved.name}`;
  if (seen.has(key)) return;
  seen.add(key);
  targets.push(resolved);
}

function inferSingleProvider(targets: SchemaTarget[]): void {
  const providers = new Set(
    targets
      .filter((target) => target.provider.certainty === 'definitive')
      .map((target) => target.provider.provider)
      .filter((provider) => provider !== undefined)
  );
  if (providers.size !== 1) return;
  const [provider] = providers;
  for (const target of targets) {
    if (target.provider.certainty === 'ambiguous') {
      target.provider = { certainty: 'inferred', provider };
    }
  }
}

function formatSpan(span: { file: string; line: number; col: number }): string {
  return `${span.file}:${span.line}:${span.col}`;
}
