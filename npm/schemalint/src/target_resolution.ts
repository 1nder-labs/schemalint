import type * as ts from 'typescript';

import type {
  EnvelopeField,
  ProviderResolution,
  TargetSpan,
} from './sdk_adapters.js';
import { stringLiteralText } from './object_properties.js';
import { staticAlternatives, unwrapExpression } from './static_expression.js';

export interface TargetMetadata {
  adapterModule: string;
  canonicalKind: string;
  provider: ProviderResolution;
  envelope: Record<string, EnvelopeField>;
  usageSpan: TargetSpan;
}

export interface TargetExpression {
  name: string;
  sourceFile: ts.SourceFile;
  expression: ts.Expression;
  metadata: TargetMetadata;
}

export function namedTarget(
  api: string,
  expression: ts.Expression,
  sourceFile: ts.SourceFile,
  tsModule: typeof ts,
  explicitName: string | undefined,
  metadata: TargetMetadata
): TargetExpression {
  const { line } = sourceFile.getLineAndCharacterOfPosition(
    expression.getStart(sourceFile)
  );
  const expr = unwrapExpression(expression, tsModule);
  const suffix =
    explicitName ??
    (tsModule.isIdentifier(expr) ? expr.text : `inline:${line + 1}`);
  return {
    name: `${api}:${suffix}`,
    sourceFile,
    expression,
    metadata,
  };
}

export function spanFor(node: ts.Node, sourceFile: ts.SourceFile): TargetSpan {
  const { line, character } = sourceFile.getLineAndCharacterOfPosition(
    node.getStart(sourceFile)
  );
  return { file: sourceFile.fileName, line: line + 1, col: character + 1 };
}

export function stringValueFromExpression(
  expr: ts.Expression | undefined,
  checker: ts.TypeChecker,
  tsModule: typeof ts
): string | undefined {
  const values = staticAlternatives(expr, checker, tsModule).map((alternative) =>
    stringLiteralText(alternative, tsModule)
  );
  if (values.length === 0 || values.some((value) => value === undefined)) {
    return undefined;
  }
  const distinct = new Set(values);
  return distinct.size === 1 ? values[0] : undefined;
}
