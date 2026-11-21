# Contributing to HyperLoad

Thanks for helping build HyperLoad. Contributions are welcome from developers working on networking, systems programming, SDKs, testing, documentation, and platform integration.

## Before starting

1. Check existing Issues and Discussions.
2. For substantial changes, open an issue first to agree on the design.
3. Keep changes focused and avoid unrelated formatting churn.
4. Never commit credentials, private URLs, proprietary game files, or copyrighted test assets.

## Local checks

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

## Pull request checklist

- [ ] Explain the problem and proposed solution.
- [ ] Add or update tests.
- [ ] Update documentation and CLI help where applicable.
- [ ] Handle errors without panics in library code.
- [ ] Consider interruption, retries, disk-full conditions, and partial writes.
- [ ] Do not weaken TLS verification or integrity checks.
- [ ] Confirm the change does not bypass DRM or access controls.

## Design principles

- The Rust core is the source of truth for transfer behavior.
- Be conservative with concurrency and respect origin server limits.
- Resume must validate that partial data is still compatible.
- Never assume that every HTTP server supports byte ranges.
- Keep platform-specific code behind clear interfaces.
- Prefer safe defaults and explicit opt-ins.
