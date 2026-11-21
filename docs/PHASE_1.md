# Phase 1 — Reliable Core

## Goal

Deliver a reusable Rust downloader that can safely transfer large HTTP/HTTPS resources, use byte ranges when available, recover completed chunks after interruption, and verify output integrity.

## Included in this starter

1. Resource probing and byte-range capability detection.
2. Parallel chunk transfers with bounded concurrency.
3. Sequential streaming fallback.
4. Retry attempts for failed chunks.
5. Persistent manifest and chunk files for range-based transfers.
6. Resume validation based on URL, resource size, chunk size, and chunk lengths.
7. SHA-256 verification.
8. Assembly to a temporary file and final rename.
9. CLI and automated CI checks.

## Known limitations / next hardening tasks

- Persist and validate ETag / Last-Modified validators to detect changed remote objects.
- Add cancellation tokens and graceful Ctrl+C checkpointing.
- Add structured progress events and rate calculations.
- Add disk-space preflight and configurable storage directory.
- Add local HTTP integration tests for 200, 206, 416, truncated bodies, redirects, and changing content.
- Consider per-chunk checksums in the manifest.
- Add safe handling for `Content-Disposition` filenames.
- Add proxy configuration and explicit redirect policy.
- Add platform-specific file locking to prevent two processes writing the same target.
- Add fuzzing and property-based tests for manifest parsing and range arithmetic.

A Phase 1 release should not be called production-ready until the above test and hardening tasks have been completed.
