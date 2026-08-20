'use strict';

const compatibility = require('../npm/schemalint/src/__tests__/fixtures/sdk-version-matrix.json');
const { conditionalFixtures } = require('./packed-sdk-conditional-fixtures.cjs');

const OPENAI = { certainty: 'definitive', provider: 'openai' };
const ANTHROPIC = { certainty: 'definitive', provider: 'anthropic' };
const AMBIGUOUS = { certainty: 'ambiguous' };
const NAME_64 = 'a'.repeat(64);
const NAME_65 = 'b'.repeat(65);

function field(required, value, lineMarker, token = `'${value}'`) {
  return { required, value, lineMarker, token };
}
function model(kind, provider, envelope = {}) {
  return { kind, provider, envelope };
}
function packageSpec(packageName, surface, row) {
  const entry = compatibility.packages.find(
    (candidate) => candidate.package === packageName && candidate.surfaces.includes(surface)
  );
  if (!entry) throw new Error(`SDK matrix has no ${packageName}:${surface}`);
  return `${packageName}@${entry[row]}`;
}

const legacySource = [
  "import { z } from 'zod';",
  "import * as aiSdk from 'ai';",
  "import { Output as Result, dynamicTool as makeDynamicTool } from 'ai';",
  "import { zodResponseFormat, zodFunction as makeFunction } from 'openai/helpers/zod';",
  "import * as anthropicBeta from '@anthropic-ai/sdk/helpers/beta/zod';",
  '',
  'const Shared = z.object({ value: z.string() });',
  '',
  "aiSdk.Output.object({ name: 'legacy_object', description: 'legacy object', schema: Shared });",
  "Result.array({ name: 'legacy_array', element: z.object({ item: z.string() }) });",
  "makeDynamicTool({ description: 'legacy dynamic', inputSchema: Shared, execute: async () => ({}) });",
  "zodResponseFormat(Shared, 'legacy_response');",
  "makeFunction({ name: 'legacy_function', parameters: Shared });",
  "anthropicBeta.betaZodTool({ name: 'legacy_anthropic_tool', description: 'legacy tool', inputSchema: Shared, run: async () => 'ok' });",
  'const unresolvedName = String(Date.now());',
  'zodResponseFormat(Shared, unresolvedName);',
].join('\n');
const structuredFloorSource = [
  "import { z } from 'zod';",
  "import { Output } from 'ai';",
  "import { zodResponseFormat as responseAlias, zodTextFormat } from 'openai/helpers/zod';",
  "import * as openaiZod from 'openai/helpers/zod';",
  "import { zodOutputFormat as outputAlias } from '@anthropic-ai/sdk/helpers/zod';",
  "import * as anthropicZod from '@anthropic-ai/sdk/helpers/zod';",
  '',
  'const Shared = z.object({ value: z.string() });',
  '',
  "Output.object({ name: 'floor_object', schema: Shared });",
  "responseAlias(Shared, 'floor_response');",
  "zodTextFormat(Shared, 'floor_text');",
  "openaiZod.zodTextFormat(Shared, 'floor_namespace_text');",
  'outputAlias(Shared);',
  'anthropicZod.zodOutputFormat(Shared);',
].join('\n');
const currentPartialSource = [
  "import { z } from 'zod';",
  "import * as aiSdk from 'ai';",
  "import { Output as Result, dynamicTool as makeDynamicTool } from 'ai';",
  "import { zodResponseFormat, zodTextFormat as openaiText, zodFunction } from 'openai/helpers/zod';",
  "import * as anthropicZod from '@anthropic-ai/sdk/helpers/zod';",
  "import { betaZodTool as makeBetaTool } from '@anthropic-ai/sdk/helpers/beta/zod';",
  '',
  'const Clean = z.object({ value: z.string() });',
  '',
  "aiSdk.Output.object({ name: 'current_object', description: 'current object', schema: Clean });",
  "Result.array({ name: 'current_array', element: z.object({ item: z.string() }) });",
  "makeDynamicTool({ description: 'current dynamic', inputSchema: Clean, execute: async () => ({}) });",
  "zodResponseFormat(Clean, '');",
  `zodResponseFormat(Clean, '${NAME_64}');`,
  `zodResponseFormat(Clean, '${NAME_65}');`,
  "openaiText(Clean, 'bad name');",
  "zodFunction({ name: 'current_function', parameters: Clean });",
  "makeBetaTool({ name: 'current_anthropic_tool', description: 'current tool', inputSchema: Clean, run: async () => 'ok' });",
  'anthropicZod.zodOutputFormat(Clean);',
  'const unresolvedName = String(Date.now());',
  'openaiText(Clean, unresolvedName);',
].join('\n');
const currentCompleteSource = [
  "import { z } from 'zod';",
  "import { zodResponseFormat as openaiResponse } from 'openai/helpers/zod';",
  "import * as anthropicZod from '@anthropic-ai/sdk/helpers/zod';",
  '',
  'const SharedRestricted = z.object({ count: z.number().min(1) });',
  '',
  'anthropicZod.zodOutputFormat(SharedRestricted);',
  "openaiResponse(SharedRestricted, 'complete_openai');",
].join('\n');

const minimumZod = `zod@${compatibility.schema_runtimes.zod_v3}`;
const currentPackages = [
  ...new Set(compatibility.packages.map((entry) => `${entry.package}@${entry.current}`)),
  `zod@${compatibility.schema_runtimes.zod_v4_current}`,
];
const matrix = {
  minimum: [
    {
      label: 'legacy-floors',
      packages: [
        packageSpec('ai', 'Output.object', 'minimum'),
        packageSpec('openai', 'zodResponseFormat', 'minimum'),
        packageSpec('@anthropic-ai/sdk', 'betaZodTool', 'minimum'),
        minimumZod,
      ],
      fixtures: [{
        file: 'legacy.ts', source: legacySource,
        models: [
          model('ai.Output.object', AMBIGUOUS, { name: field(false, 'legacy_object', "aiSdk.Output.object({ name: 'legacy_object'"), description: field(false, 'legacy object', "description: 'legacy object'") }),
          model('ai.Output.array', AMBIGUOUS, { name: field(false, 'legacy_array', "Result.array({ name: 'legacy_array'") }),
          model('ai.dynamicTool', AMBIGUOUS, { description: field(false, 'legacy dynamic', "description: 'legacy dynamic'") }),
          model('openai.zodResponseFormat', OPENAI, { name: field(true, 'legacy_response', "zodResponseFormat(Shared, 'legacy_response')") }),
          model('openai.zodFunction', OPENAI, { name: field(true, 'legacy_function', "name: 'legacy_function'") }),
          model('anthropic.betaZodTool', ANTHROPIC, { name: field(true, 'legacy_anthropic_tool', "name: 'legacy_anthropic_tool'") }),
        ],
        failure: { target: 'openai.zodResponseFormat', lineMarker: 'zodResponseFormat(Shared, unresolvedName);', token: 'unresolvedName' },
      }],
    },
    {
      label: 'structured-output-floors',
      packages: [
        packageSpec('ai', 'Output.object', 'minimum'),
        packageSpec('openai', 'zodTextFormat', 'minimum'),
        packageSpec('@anthropic-ai/sdk', 'zodOutputFormat', 'minimum'),
        minimumZod,
      ],
      fixtures: [{
        file: 'structured.ts', source: structuredFloorSource,
        models: [
          model('ai.Output.object', AMBIGUOUS, { name: field(false, 'floor_object', "Output.object({ name: 'floor_object'") }),
          model('openai.zodResponseFormat', OPENAI, { name: field(true, 'floor_response', "responseAlias(Shared, 'floor_response')") }),
          model('openai.zodTextFormat', OPENAI, { name: field(true, 'floor_text', "zodTextFormat(Shared, 'floor_text')") }),
          model('openai.zodTextFormat', OPENAI, { name: field(true, 'floor_namespace_text', "openaiZod.zodTextFormat(Shared, 'floor_namespace_text')") }),
          model('anthropic.zodOutputFormat', ANTHROPIC),
          model('anthropic.zodOutputFormat', ANTHROPIC),
        ],
      }],
    },
  ],
  current: [{
    label: 'current', packages: currentPackages,
    fixtures: [
      ...conditionalFixtures({ field, model, OPENAI, AMBIGUOUS }),
      {
        file: 'partial.ts', source: currentPartialSource,
        models: [
          model('ai.Output.object', AMBIGUOUS, { name: field(false, 'current_object', "aiSdk.Output.object({ name: 'current_object'"), description: field(false, 'current object', "description: 'current object'") }),
          model('ai.Output.array', AMBIGUOUS, { name: field(false, 'current_array', "Result.array({ name: 'current_array'") }),
          model('ai.dynamicTool', AMBIGUOUS, { description: field(false, 'current dynamic', "description: 'current dynamic'") }),
          model('openai.zodResponseFormat', OPENAI, { name: field(true, '', "zodResponseFormat(Clean, '');", "''") }),
          model('openai.zodResponseFormat', OPENAI, { name: field(true, NAME_64, `zodResponseFormat(Clean, '${NAME_64}')`) }),
          model('openai.zodResponseFormat', OPENAI, { name: field(true, NAME_65, `zodResponseFormat(Clean, '${NAME_65}')`) }),
          model('openai.zodTextFormat', OPENAI, { name: field(true, 'bad name', "openaiText(Clean, 'bad name')") }),
          model('openai.zodFunction', OPENAI, { name: field(true, 'current_function', "name: 'current_function'") }),
          model('anthropic.betaZodTool', ANTHROPIC, { name: field(true, 'current_anthropic_tool', "name: 'current_anthropic_tool'") }),
          model('anthropic.zodOutputFormat', ANTHROPIC),
        ],
        failure: { target: 'openai.zodTextFormat', lineMarker: 'openaiText(Clean, unresolvedName);', token: 'unresolvedName' },
        cli: {
          coverage: { status: 'partial', attempted: 11, excluded: 0, discovered: 10, checked: 7, failed: 4 }, errors: 3,
          diagnostics: [
            { code: 'OAI-S-envelope-name', profile: 'openai.so.2026-04-30', pointer: '', lineMarker: "zodResponseFormat(Clean, '');", token: "''" },
            { code: 'OAI-S-envelope-name', profile: 'openai.so.2026-04-30', pointer: '', lineMarker: `zodResponseFormat(Clean, '${NAME_65}')`, token: `'${NAME_65}'` },
            { code: 'OAI-S-envelope-name', profile: 'openai.so.2026-04-30', pointer: '', lineMarker: "openaiText(Clean, 'bad name')", token: "'bad name'" },
          ],
          failureKinds: ['ai.Output.array', 'ai.Output.object', 'ai.dynamicTool', 'openai.zodTextFormat'],
        },
      },
      {
        file: 'complete.ts', source: currentCompleteSource,
        models: [
          model('anthropic.zodOutputFormat', ANTHROPIC),
          model('openai.zodResponseFormat', OPENAI, { name: field(true, 'complete_openai', "openaiResponse(SharedRestricted, 'complete_openai')") }),
        ],
        cli: {
          coverage: { status: 'complete', attempted: 2, excluded: 0, discovered: 2, checked: 2, failed: 0 }, errors: 1,
          diagnostics: [{ code: 'ANT-K-minimum', profile: 'anthropic.so.2026-04-30', pointer: '/properties/count', lineMarker: 'const SharedRestricted', token: 'count', lineOnly: true }],
          failureKinds: [],
        },
      },
    ],
  }],
};

module.exports = { compatibility, matrix };
