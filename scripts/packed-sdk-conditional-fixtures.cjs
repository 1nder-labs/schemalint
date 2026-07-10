'use strict';

function conditionalFixtures({ field, model, OPENAI, AMBIGUOUS }) {
  const optionsSource = [
    "import { Output } from 'ai';",
    "import { zodTextFormat } from 'openai/helpers/zod';",
    "import { z } from 'zod';",
    '',
    'const First = z.object({ first: z.string() });',
    'const Second = z.object({ second: z.string() });',
    '',
    'Output.object(Math.random() > 0.5 ? { name: \'different_options\', schema: First } : { name: \'different_options\', schema: Second });',
    'Output.object(Math.random() > 0.5 ? { name: \'same_options\', schema: First } : { name: \'same_options\', schema: First });',
    "zodTextFormat(First, 'provider_anchor');",
  ].join('\n');
  const schemaSource = [
    "import { zodTextFormat } from 'openai/helpers/zod';",
    "import { z } from 'zod';",
    '',
    'const First = z.object({ first: z.string() });',
    'const Second = z.object({ second: z.string() });',
    'const Selected = Math.random() > 0.5 ? First : Second;',
    '',
    "zodTextFormat(Selected, 'selected_schema');",
    "zodTextFormat(Math.random() > 0.5 ? First : First, 'same_schema');",
  ].join('\n');
  const nameSource = [
    "import { zodTextFormat } from 'openai/helpers/zod';",
    "import { z } from 'zod';",
    '',
    'const First = z.object({ first: z.string() });',
    '',
    "zodTextFormat(First, Math.random() > 0.5 ? 'first_name' : 'second_name');",
    "zodTextFormat(First, Math.random() > 0.5 ? 'same_name' : 'same_name');",
  ].join('\n');

  return [
    {
      file: 'conditional-options.ts',
      source: optionsSource,
      models: [
        model('ai.Output.object', AMBIGUOUS, {
          name: field(false, 'same_options', "name: 'same_options'"),
        }),
        model('openai.zodTextFormat', OPENAI, {
          name: field(true, 'provider_anchor', "zodTextFormat(First, 'provider_anchor')"),
        }),
      ],
      failure: {
        target: 'ai.Output.object',
        message: 'required schema metadata',
        lineMarker: "name: 'different_options'",
        token: 'Output.object',
      },
      cli: {
        coverage: { status: 'partial', attempted: 3, excluded: 0, discovered: 2, checked: 2, failed: 1 },
        errors: 0,
        diagnostics: [],
        failureKinds: ['ai.Output.object'],
      },
    },
    {
      file: 'conditional-schema.ts',
      source: schemaSource,
      models: [
        model('openai.zodTextFormat', OPENAI, {
          name: field(true, 'same_schema', "zodTextFormat(Math.random() > 0.5 ? First : First, 'same_schema')"),
        }),
      ],
      failure: {
        target: 'openai.zodTextFormat',
        message: 'required schema metadata',
        lineMarker: "zodTextFormat(Selected, 'selected_schema')",
        token: 'zodTextFormat',
      },
      cli: {
        coverage: { status: 'partial', attempted: 2, excluded: 0, discovered: 1, checked: 1, failed: 1 },
        errors: 0,
        diagnostics: [],
        failureKinds: ['openai.zodTextFormat'],
      },
    },
    {
      file: 'conditional-name.ts',
      source: nameSource,
      models: [
        model('openai.zodTextFormat', OPENAI, {
          name: field(
            true,
            'same_name',
            "zodTextFormat(First, Math.random() > 0.5 ? 'same_name' : 'same_name')",
            'Math.random()'
          ),
        }),
      ],
      failure: {
        target: 'openai.zodTextFormat',
        lineMarker: "zodTextFormat(First, Math.random() > 0.5 ? 'first_name' : 'second_name')",
        token: 'Math.random()',
      },
      cli: {
        coverage: { status: 'partial', attempted: 2, excluded: 0, discovered: 1, checked: 1, failed: 1 },
        errors: 0,
        diagnostics: [],
        failureKinds: ['openai.zodTextFormat'],
      },
    },
  ];
}

module.exports = { conditionalFixtures };
