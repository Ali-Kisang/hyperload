# HyperLoad Roadmap

## Phase 1 — Reliable core (current)
Rust engine, parallel HTTP ranges, fallback, retries, resumable chunk state, SHA-256, CLI.

## Phase 2 — Engine hardening
ETag / Last-Modified validation, cancellation, progress events, disk-space checks, file locking, comprehensive local-server integration tests, benchmarks.

## Phase 3 — Stable integration interfaces
C ABI and UniFFI bindings, Node.js SDK, Python bindings, versioned API contracts, typed progress events.

## Phase 4 — Local background service
Axum REST API and WebSocket events, loopback-only binding, client authentication, permissions, installer and service lifecycle.

## Phase 5 — Cross-platform clients
Windows, Linux and macOS desktop integrations; Android native foreground/background work; iOS background URLSession integration with platform constraints.

## Phase 6 — Ecosystem
Tauri desktop UI, React Native client, developer documentation, examples, package publishing, plugin adapters for officially supported applications.

## Gaming integrations

Only integrate with launchers through documented or otherwise authorized interfaces. Do not bypass DRM, launcher authentication, encrypted manifests, entitlement checks, or publisher integrity mechanisms. A generic HTTP adapter does not imply compatibility with Steam, Epic, or Rockstar launchers.
