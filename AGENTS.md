# Basecamp CLI Agent Guide

This file defines how any personal agent should build and maintain this project.

## Codex Loading Convention

- Codex auto-loads `AGENTS.md` from the repo root and from directories along the working path.
- Feature guidance lives under `docs/agent/features/<feature>/`.
- Each feature folder must contain:
  - `api.md`
  - `cli.md`
  - `code.md`
- Before implementing a feature, read that feature's `api.md`, `cli.md`, and `code.md`.
- Current Stage 1 feature:
  - `docs/agent/features/auth/api.md`
  - `docs/agent/features/auth/cli.md`
  - `docs/agent/features/auth/code.md`

## Mission

Build a small-footprint Basecamp CLI in Rust.

Stage 1 scope is the auth feature via:

```bash
basecamp integration set
basecamp integration show
basecamp integration clear
basecamp login
basecamp logout
```

## Required Sources of Truth

- Rust implementation style: <https://github.com/openai/codex/tree/main/codex-rs>
- Basecamp API behavior: <https://github.com/basecamp/bc3-api>

Do not invent API behavior outside those sources.

## Rust Standards

- Use Rust `1.93.1` and keep code compatible with that version.
- Use Rust edition `2024`.
- Prefer small binaries and low runtime overhead.
- Prefer `Result`-based error handling and typed errors.
- Do not use `unwrap()` or `expect()` in production paths.
- Keep formatting and linting strict (`cargo fmt`, `cargo clippy -D warnings`).

## Dependencies

- Any new dependency must be proposed and approved by the user before being added to `Cargo.toml`.
- Every proposal must include:
  - purpose in this project
  - expected binary/runtime impact
  - maintenance and security considerations
  - why stdlib or existing dependencies are not enough
- Approved baseline dependencies:
  - `clap = { version = "4.5.58", features = ["derive"] }`
  - `colored = "3.0.0"`

## Style Alignment with codex-rs

- Keep modules focused and composable.
- Prefer explicit types at API boundaries.
- Use inline format arguments (`format!("{value}")`).
- Prefer exhaustive `match` when practical.
- Avoid unnecessary helper functions used only once.
- Use clear naming and stable CLI UX.

## Stage 1 Functional Contract (Auth Feature)

The auth feature must:

1. Support integration credential management via `basecamp integration set/show/clear`.
2. Start OAuth 2.0 Authorization Code flow for login.
3. Open browser to Basecamp Launchpad authorization URL.
4. Receive `code` on callback redirect URI.
5. Exchange `code` for `access_token` + `refresh_token`.
6. Call `GET /authorization.json` with bearer token.
7. Select an account where `product == "bc3"`.
8. Persist credentials securely for later commands.
9. Support logout by clearing local session tokens.

## Security Requirements

- Never print secrets or tokens in normal logs.
- Persist refresh/access tokens in OS keychain when possible.
- Keep non-secret metadata (selected account, URLs) in local config.
- Support token refresh without forcing re-login when refresh token is valid.

## Deliverable for Stage 1

A working auth command set and docs in:

- `AGENTS.md`
- `docs/agent/features/auth/api.md`
- `docs/agent/features/auth/cli.md`
- `docs/agent/features/auth/code.md`
