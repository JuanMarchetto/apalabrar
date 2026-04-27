# Security Policy

## Reporting a vulnerability

Please **do not** open a public GitHub issue for security-sensitive findings.

Email: **juanpatriciomarchetto@gmail.com** with subject prefix `[SECURITY]`.

Include:
- Affected component (crate / package)
- Affected versions
- Reproduction steps
- Impact assessment

You should receive an acknowledgement within 7 days. Coordinated disclosure window is 90 days unless agreed otherwise.

## Scope

- The Rust + WASM editor core
- Plugin sandbox isolation
- Storage encryption (cloud sync, when v1 ships)
- Auth (when v1 ships)
- The hosted SaaS (when it exists)

## Out of scope

- Third-party plugins (report to plugin authors)
- Issues that require physical access to the user's device
- Social engineering of the user
- Browser bugs (report to the browser vendor)

## Secure design notes (informational)

- Plugins run inside a WASM Component Model sandbox with capability-based interfaces. They have no DOM access by default.
- AGPL components are never bundled in the client; they run as separate processes when present at all.
- All cloud sync is end-to-end encrypted before transmission (v1+).
- BYO-key cloud AI: user's API keys are stored in OPFS (origin-isolated) and never sent to Apalabrar servers.
