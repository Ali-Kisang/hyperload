# Security Policy

## Reporting a vulnerability

Please do not publish exploitable vulnerability details in a public issue. Contact the repository maintainers privately through the GitHub Security Advisories feature (Security → Advisories → Report a vulnerability), once enabled.

Include:
- Affected version/commit.
- Impact and realistic attack scenario.
- Reproduction steps or a proof of concept that does not expose real secrets.
- Suggested mitigation, if known.

## Scope

The project is early alpha. Do not use it as a privileged system service or for untrusted multi-user workloads.

Potentially sensitive areas include URL handling, redirect behavior, destination paths, local API authentication, archive extraction (not currently implemented), and resume-manifest integrity.

## Security expectations

- TLS certificate validation must remain enabled.
- Never log authorization headers or full signed URLs.
- Future service/API releases must authenticate local clients and validate destination paths.
- Avoid binding future local APIs to public interfaces by default.
