import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { adapterFor } from '../sdk_adapters.js';

interface VersionFixture {
  packages: Array<{
    package: string;
    minimum: string;
    current: string;
    module: string;
    surfaces: string[];
  }>;
  deprecated_in_schemalint_1_x: {
    removal: string;
    surfaces: string[];
  };
}

const fixturePath = path.join(
  path.dirname(fileURLToPath(import.meta.url)),
  'fixtures',
  'sdk-version-matrix.json'
);
const fixture = JSON.parse(readFileSync(fixturePath, 'utf8')) as VersionFixture;

describe('declared SDK version matrix', () => {
  it('maps every versioned fixture surface to a canonical adapter', () => {
    for (const sdk of fixture.packages) {
      expect(sdk.minimum, `${sdk.package} minimum`).toMatch(/^\d+\.\d+\.\d+$/);
      expect(sdk.current, `${sdk.package} current`).toMatch(/^\d+\.\d+\.\d+$/);
      for (const surface of sdk.surfaces) {
        expect(adapterFor(sdk.module, surface), `${sdk.module}:${surface}`).toBeDefined();
      }
    }
  });

  it('keeps deprecated adapters on the documented 2.0 removal boundary', () => {
    expect(fixture.deprecated_in_schemalint_1_x.removal).toBe('2.0');
    expect(fixture.deprecated_in_schemalint_1_x.surfaces).toEqual(
      expect.arrayContaining([
        'generateObject',
        'streamObject',
        'tool',
        'zodFunction',
        'betaZodTool',
      ])
    );
  });
});
