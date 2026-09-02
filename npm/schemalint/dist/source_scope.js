import { statSync } from 'node:fs';
import path from 'node:path';
const SOURCE_EXTENSIONS = '{ts,tsx,mts,cts}';
/**
 * Turn a `--source` value into a file matcher. A literal path to an existing
 * directory expands to every TypeScript file beneath it; a literal path to an
 * existing file matches that file alone; anything else is a glob.
 */
export function resolveSourceScope(source, matcher, projectRoot) {
    // On disk decides: no real file is named `*.ts`, while a real directory
    // can be named `(marketing)`, so a glob-syntax check would misread it.
    const explicit = existingKind(source, projectRoot);
    const trimmed = source.replace(/\/+$/, '');
    const pattern = explicit === 'directory' ? `${trimmed}/**/*.${SOURCE_EXTENSIONS}` : source;
    return {
        pattern,
        isMatch: matcher(pattern, { dot: true }),
        explicit: explicit !== false,
    };
}
function existingKind(source, projectRoot) {
    try {
        const stat = statSync(path.resolve(projectRoot, source));
        if (stat.isDirectory())
            return 'directory';
        if (stat.isFile())
            return 'file';
    }
    catch {
        // Not on disk: treat as a glob and let the glob-matching path explain.
    }
    return false;
}
//# sourceMappingURL=source_scope.js.map