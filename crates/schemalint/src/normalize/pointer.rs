/// Escape a single JSON Pointer (RFC 6901) segment.
///
/// `~` is replaced with `~0` first, then `/` is replaced with `~1`. The order
/// matters: escaping `/` first would introduce new `~1` sequences that the
/// `~` step would then re-escape, corrupting any segment that already
/// contained a literal `~1`.
///
/// Apply this only to segments built from a user-controlled key (a
/// `properties`, `patternProperties`, `dependentSchemas`, `$defs`, or
/// `definitions` name). Numeric indices and fixed literal segments never
/// need it.
/// True for a top-level `$defs`/`definitions` entry: a pointer of exactly the
/// shape `/$defs/<name>` or `/definitions/<name>` with no further segment.
///
/// These are the pointers `normalize::build_defs` mints, and the only cycle
/// participants a schema author can name directly, which is why a cycle
/// containing one reports there.
pub fn is_defs_entry(pointer: &str) -> bool {
    let rest = pointer
        .strip_prefix("/\u{24}defs/")
        .or_else(|| pointer.strip_prefix("/definitions/"));
    matches!(rest, Some(rest) if !rest.is_empty() && !rest.contains('/'))
}

pub fn escape_pointer_segment(segment: &str) -> String {
    if !segment.contains('~') && !segment.contains('/') {
        return segment.to_string();
    }
    segment.replace('~', "~0").replace('/', "~1")
}
