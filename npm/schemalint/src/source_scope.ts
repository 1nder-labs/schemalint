import { realpathSync, statSync } from 'node:fs';
import path from 'node:path';

import type picomatch from 'picomatch';

export interface SourceScope {
  /** The glob actually matched against project files. */
  pattern: string;
  isMatch: (relativePath: string) => boolean;
  /** Real path of the literal file or directory the source names, if it names one. */
  target?: { kind: 'file' | 'directory'; absolute: string };
}

const SOURCE_EXTENSIONS = '{ts,tsx,mts,cts}';

/**
 * Turn a `--source` value into a file matcher. A literal path to an existing
 * directory expands to every TypeScript file beneath it; a literal path to an
 * existing file matches that file alone; anything else is a glob.
 */
export function resolveSourceScope(
  source: string,
  matcher: typeof picomatch,
  projectRoot: string
): SourceScope {
  // On disk decides: no real file is named `*.ts`, while a real directory
  // can be named `(marketing)`, so a glob-syntax check would misread it.
  const kind = existingKind(source, projectRoot);
  const trimmed = source.replace(/\/+$/, '');
  const pattern = kind === 'directory' ? `${trimmed}/**/*.${SOURCE_EXTENSIONS}` : source;
  return {
    pattern,
    isMatch: matcher(pattern, { dot: true }) as (input: string) => boolean,
    ...(kind && {
      target: {
        kind,
        absolute: realpathSync(path.resolve(projectRoot, source)),
      },
    }),
  };
}

function existingKind(
  source: string,
  projectRoot: string
): 'file' | 'directory' | false {
  try {
    const stat = statSync(path.resolve(projectRoot, source));
    if (stat.isDirectory()) return 'directory';
    if (stat.isFile()) return 'file';
  } catch {
    // Not on disk: treat as a glob and let the glob-matching path explain.
  }
  return false;
}
