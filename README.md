# HyperLoad

**An open-source, embeddable download engine focused on reliable large-file transfers.**

HyperLoad is intended to provide a reusable download core that applications can integrate through language SDKs or a local service. The first milestone focuses on a safe, resumable Rust engine for ordinary HTTP/HTTPS resources.

> **Project status:** early alpha / Phase 1 foundation. The Rust core and CLI are the active implementation. Mobile bindings, background services, launcher adapters, and production security hardening are future milestones—not features to assume are already available.

## What Phase 1 includes

- HTTP/HTTPS downloads using Rust and Tokio.
- Parallel byte-range downloads when the origin server supports them.
- Sequential streaming fallback when range requests are unavailable.
- Per-chunk retry handling.
- Persistent chunk files and a JSON resume manifest.
- Resume completed byte-range chunks after process interruption (for compatible unchanged URLs/resources); sequential fallback restarts from the beginning.
- Optional SHA-256 verification.
- Temporary output followed by final rename.
- CLI, unit tests, contribution guidance, and CI workflow.

## What it does not promise

HyperLoad cannot increase the bandwidth purchased from an ISP, bypass server throttling, defeat DRM, evade authentication, or make an incompatible launcher use a third-party engine. Parallel requests can be slower or disallowed by some origins. Use only URLs and content you are authorized to download. Game launchers often use proprietary manifests, encrypted packages, authentication, and integrity systems; launcher integration requires an official/authorized interface.

## Requirements

- Rust stable (install from <https://rustup.rs/>).
- Git.
- Internet access for downloading crates and test resources.

## Quick start

```bash
git clone https://github.com/YOUR-ORG/hyperload.git
cd hyperload
cargo build --release
```

Download a file:

```bash
cargo run --release -- \
  "https://example.com/archive.zip" \
  --output "./downloads/archive.zip" \
  --connections 8 \
  --chunk-mib 16 \
  --retries 4
```

For servers that support byte ranges, resume is automatic when the same URL, output path, and compatible transfer settings are used again. Sequential fallback restarts from the beginning after interruption. HyperLoad stores partial chunks and its manifest in a hidden `.hyperload` directory beside the output. Do not delete that directory if you want to resume.

Optional SHA-256 verification:

```bash
cargo run --release -- \
  "https://example.com/archive.zip" \
  --output "./downloads/archive.zip" \
  --sha256 "PUT_THE_EXPECTED_64_CHARACTER_SHA256_HERE"
```

## CLI options

| Option | Default | Meaning |
|---|---:|---|
| `--output` | required | Destination file |
| `--connections` | 8 | Maximum parallel chunk requests (1–32) |
| `--chunk-mib` | 16 | Chunk size in MiB |
| `--retries` | 4 | Retries per chunk |
| `--sha256` | none | Expected SHA-256 digest |

Use a conservative connection count. More connections do not automatically mean faster downloads and may trigger server throttling.

## Resume model

For a destination such as `downloads/file.zip`, HyperLoad creates:

```text
downloads/
├── file.zip                 # appears after successful assembly
└── .hyperload/
    └── <stable-id>/
        ├── manifest.json    # URL, size, chunk size and completion state
        └── chunks/
            ├── 00000000.part
            └── 00000001.part
```

The manifest is written through a temporary file and renamed to reduce the chance of corruption. Completed chunks are re-checked against their expected lengths when resuming. A server that changes the underlying resource without a stable validator can still make a resumed transfer unsafe; for important files, provide an expected SHA-256 digest.

## Development

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Run with logs:

```bash
RUST_LOG=hyperload=debug cargo run -- "https://example.com/file.bin" --output file.bin
```

Windows PowerShell:

```powershell
$env:RUST_LOG = "hyperload=debug"
cargo run -- "https://example.com/file.bin" --output file.bin
```

## Architecture

- `crates/hyperload-core`: reusable Rust library and transfer orchestration.
- `crates/hyperload-cli`: command-line interface.
- `docs/PHASE_1.md`: Phase 1 scope and implementation notes.
- `docs/ROADMAP.md`: future milestones and platform strategy.

The core is deliberately independent of a desktop UI or launcher. Future adapters should call the core through a stable API rather than duplicating transfer logic.

## Open-source contribution

HyperLoad uses the MIT License. Read [CONTRIBUTING.md](CONTRIBUTING.md), [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md), and [SECURITY.md](SECURITY.md) before contributing.

Good first contributions:
- Add integration tests using a local HTTP test server.
- Add ETag / Last-Modified validation to resume metadata.
- Improve progress events and cancellation.
- Add Windows/macOS/Linux CI coverage.
- Benchmark chunk sizes and concurrency on controlled test fixtures.
- Improve error messages and documentation.

### Publishing to GitHub

1. Create an empty public repository named `hyperload` under your GitHub account or organization.
2. Update `repository` in the root `Cargo.toml` and replace `YOUR-ORG` in this README.
3. From this folder, run:

```bash
git init
git add .
git commit -m "Initial HyperLoad Phase 1 implementation"
git branch -M main
git remote add origin https://github.com/YOUR-ORG/hyperload.git
git push -u origin main
```

4. Enable Issues and Discussions in repository settings.
5. Add maintainers and protect `main` with required CI checks.

## Security notes

- Do not embed private tokens, passwords, or signed URLs in source code or public issue reports.
- Avoid downloading untrusted executables. Verify checksums or signatures from the publisher.
- The future local API must bind to loopback by default, authenticate clients, validate paths, and never expose an unauthenticated arbitrary-download endpoint to a network.
- Never use HyperLoad to circumvent DRM, access controls, or a service's terms.

## License

MIT. See [LICENSE](LICENSE).
