## Summary

<!-- What does this PR change and why? One or two sentences. -->

## Related issue

Closes #

## Checklist

- [ ] Tests added or updated for the changed behavior
- [ ] `cargo test --workspace` passes locally
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` is clean
- [ ] `cargo fmt --all` applied (no diff)
- [ ] PR title follows [Conventional Commits](https://www.conventionalcommits.org/) (`feat:`, `fix:`, `chore:`, etc.)
- [ ] Docs updated if behavior, flags, or rule output changed
- [ ] For rule / profile changes: snapshot files updated (`cargo test --workspace -- --nocapture` or `UPDATE_EXPECT=1`)
