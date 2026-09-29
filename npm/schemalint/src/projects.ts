import { realpathSync } from 'node:fs';
import path from 'node:path';

import type picomatch from 'picomatch';
import type * as ts from 'typescript';

import type { SourceScope } from './source_scope.js';

const SOURCE_EXTENSIONS = ['.ts', '.tsx', '.mts', '.cts'];
const DECLARATION_FILE = /\.d\.[cm]?ts$/;

/** Compiler options for files with no tsconfig.json above them. */
function defaultCompilerOptions(tsModule: typeof ts): ts.CompilerOptions {
  return {
    target: tsModule.ScriptTarget.ES2022,
    module: tsModule.ModuleKind.ESNext,
    moduleResolution: tsModule.ModuleResolutionKind.Bundler,
    strict: true,
    esModuleInterop: true,
    skipLibCheck: true,
    resolveJsonModule: true,
  };
}

/** One TypeScript program's worth of source files sharing compiler options. */
export interface ProjectGroup {
  /** The nearest tsconfig.json, or undefined for the synthesized default. */
  configPath: string | undefined;
  compilerOptions: ts.CompilerOptions;
  files: string[];
}

function realPath(file: string): string {
  try {
    return realpathSync(file);
  } catch {
    return file;
  }
}

/**
 * Every TypeScript source file the scope names, as real paths.
 *
 * Listed from disk rather than from a tsconfig's include list, so a scope can
 * reach packages whose tsconfig is not the one at the working directory.
 * Real paths make `/tmp/x` and `/private/tmp/x` (a macOS symlink) the same
 * file when the program's own module resolution reports the resolved one.
 */
export function listScopeFiles(
  scope: SourceScope,
  projectRoot: string,
  tsModule: typeof ts,
  matcher: typeof picomatch
): string[] {
  const { target } = scope;
  if (target?.kind === 'file') {
    return isSourceFile(target.absolute) ? [target.absolute] : [];
  }
  // A glob is walked only from its static base (`packages/*/src/**` walks
  // `packages`), never the whole project, so build output beside it is not
  // read. The base may sit outside the working directory (`../shared/**`).
  const root =
    target?.absolute ?? path.resolve(projectRoot, matcher.scan(scope.pattern).base);
  const found = tsModule.sys
    .readDirectory(
      root,
      SOURCE_EXTENSIONS,
      ['**/node_modules/**', '**/.git/**'],
      ['**/*']
    )
    .filter(isSourceFile)
    .map(realPath);
  if (target) return found;
  // Match the way the glob was written: absolute globs against the absolute
  // path, relative ones against the path from the working directory.
  const absolute = path.isAbsolute(scope.pattern);
  return found.filter((file) =>
    scope.isMatch(
      absolute
        ? file
        : path.relative(projectRoot, file).split(path.sep).join('/')
    )
  );
}

function isSourceFile(file: string): boolean {
  return !DECLARATION_FILE.test(file);
}

/**
 * Group files by their nearest ancestor tsconfig.json, one group per config.
 * Files with no tsconfig above them share a synthesized default group, so a
 * package that is not itself a TypeScript project is still checked.
 */
export function groupByProject(
  files: readonly string[],
  tsModule: typeof ts,
  honorExclude: boolean
): ProjectGroup[] {
  const configByDir = new Map<string, string | undefined>();
  const groups = new Map<string, ProjectGroup>();
  const excludesByConfig = new Map<string, string[]>();

  for (const file of files) {
    const dir = path.dirname(file);
    if (!configByDir.has(dir)) {
      configByDir.set(
        dir,
        tsModule.findConfigFile(dir, tsModule.sys.fileExists, 'tsconfig.json')
      );
    }
    const configPath = configByDir.get(dir);
    const key = configPath ?? '';
    let group = groups.get(key);
    if (!group) {
      group = {
        configPath,
        compilerOptions: defaultCompilerOptions(tsModule),
        files: [],
      };
      if (configPath) {
        const config = readConfig(configPath, tsModule);
        group.compilerOptions = config.options;
        excludesByConfig.set(configPath, config.exclude);
      }
      groups.set(key, group);
    }
    group.files.push(file);
  }
  if (honorExclude) {
    for (const group of groups.values()) {
      if (!group.configPath) continue;
      group.files = withoutExcluded(
        group.files,
        group.configPath,
        excludesByConfig.get(group.configPath) ?? [],
        group.compilerOptions.outDir,
        tsModule
      );
    }
  }
  return [...groups.values()].filter((group) => group.files.length > 0);
}

/**
 * Drop files the tsconfig's `exclude` (plus its `outDir`) removes, using
 * TypeScript's own matcher. Only the directories that hold `files` are
 * listed, so this never walks the whole package the way `include` would.
 */
function withoutExcluded(
  files: readonly string[],
  configPath: string,
  exclude: readonly string[],
  outDir: string | undefined,
  tsModule: typeof ts
): string[] {
  const configDir = path.dirname(configPath);
  const excludes = [...exclude, ...(outDir ? [outDir] : [])];
  if (excludes.length === 0) return [...files];
  const dirs = new Set(
    files.map((file) =>
      path.relative(configDir, path.dirname(file)).split(path.sep).join('/')
    )
  );
  const kept = new Set(
    tsModule.sys.readDirectory(
      configDir,
      SOURCE_EXTENSIONS,
      excludes,
      [...dirs].map((dir) => (dir ? `${dir}/*` : '*'))
    )
  );
  return files.filter((file) => kept.has(file));
}

function readConfig(
  configPath: string,
  tsModule: typeof ts
): { options: ts.CompilerOptions; exclude: string[] } {
  const configFile = tsModule.readConfigFile(configPath, tsModule.sys.readFile);
  if (configFile.error) {
    throw new Error(
      `Failed to read ${configPath}: ${tsModule.flattenDiagnosticMessageText(
        configFile.error.messageText,
        '\n'
      )}`
    );
  }
  const options = tsModule.parseJsonConfigFileContent(
    // Only the options matter: the file list comes from the source scope, so
    // skip the directory walk the config's own `include` would trigger.
    { ...configFile.config, files: [], include: [] },
    tsModule.sys,
    path.dirname(configPath),
    undefined,
    configPath
  ).options;
  const exclude = configFile.config.exclude;
  return {
    options,
    exclude: Array.isArray(exclude) ? (exclude as string[]) : [],
  };
}
