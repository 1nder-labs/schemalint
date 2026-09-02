import { statSync } from 'node:fs';
import path from 'node:path';

import type picomatch from 'picomatch';

export interface SourceScope {
  /** The glob actually matched against project files. */
  pattern: string;
  isMatch: (relativePath: string) => boolean;
  /**
   * The source named a literal file or directory rather than a glob. The
   * user pointed at these files, so every exported schema in them is in
   * scope, not only the ones traced to a provider call.
   */
  explicit: boolean;
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
  const explicit = !hasGlobMagic(source) && existingKind(source, projectRoot);
  const trimmed = source.replace(/\/+$/, '');
  const pattern =
    explicit === 'directory' ? `${trimmed}/**/*.${SOURCE_EXTENSIONS}` : source;
  return {
    pattern,
    isMatch: matcher(pattern, { dot: true }) as (input: string) => boolean,
    explicit: explicit !== false,
  };
}

function hasGlobMagic(source: string): boolean {
  return /[*?[\]{}()!]/.test(source);
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
