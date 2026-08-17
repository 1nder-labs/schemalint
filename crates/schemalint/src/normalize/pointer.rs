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
pub fn escape_pointer_segment(segment: &str) -> String {
    if !segment.contains('~') && !segment.contains('/') {
        return segment.to_string();
    }
    segment.replace('~', "~0").replace('/', "~1")
}
