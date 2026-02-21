# Basecamp CLI Agent Guide

This file defines how any personal agent should build and maintain this project.

## Codex Loading Convention

- Codex auto-loads `AGENTS.md` from the repo root and from directories along the working path.
- Feature guidance lives under `docs/agent/features/<feature>/`.
- Each feature folder must contain:
  - `api.md`
  - `cli.md`
  - `code.md`
- Exception for product-level features that are not CLI/API contracts:
  - a single `code.md` is acceptable (for example `distribution`).
- Before implementing a feature, read that feature's `api.md`, `cli.md`, and `code.md`.
- Current Stage 1 feature:
  - `docs/agent/features/auth/api.md`
  - `docs/agent/features/auth/cli.md`
  - `docs/agent/features/auth/code.md`
- Current Stage 2 product feature:
  - `docs/agent/features/distribution/code.md`

## Mission

Build a small-footprint Basecamp CLI in Rust.

## Internal Basecamp SDK Layer

- Keep Basecamp API wiring in `src/basecamp/*`.
- `src/basecamp/client.rs` is the single place for:
  - Basecamp URL construction
  - bearer-token request setup
  - HTTP status -> `AppError` mapping
  - request/response decode error handling
- Feature command modules in `src/features/*` should focus on:
  - CLI args validation
  - interactive prompt flow
  - output rendering (human/JSON)
  - orchestration of SDK calls
- Do not call Basecamp endpoints directly from feature modules with ad-hoc `reqwest` code.
- Grow the internal SDK incrementally:
  - only add endpoints and models required by active CLI features
  - avoid speculative/full-surface SDK generation

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
- Approved Stage 1 auth dependencies:
  - `oauth2 = { version = "5", default-features = false, features = ["reqwest", "rustls-tls"] }`
  - `reqwest = { version = "0.12.24", default-features = false, features = ["json", "rustls-tls"] }`
  - `tokio = { version = "1.48.0", features = ["macros", "rt-multi-thread"] }`
  - `serde = { version = "1.0.228", features = ["derive"] }`
  - `serde_json = "1.0.145"`
  - `url = "2.5.7"`
  - `inquire = "0.9.3"`
- Approved Stage 1 auth secret-storage dependencies:
  - `keyring = { version = "3.6.3", default-features = false, features = ["crypto-rust"] }`
  - `age = "0.11.2"`
  - `rand = "0.10.0"`
  - `sha2 = "0.10.9"`
  - `base64 = "0.22.1"`

## Style Alignment with codex-rs

- Keep modules focused and composable.
- Prefer explicit types at API boundaries.
- Use inline format arguments (`format!("{value}")`).
- Prefer exhaustive `match` when practical.
- Avoid unnecessary helper functions used only once.
- Use clear naming and stable CLI UX.

## CLI Color Rules

- Use a slightly dim prompt color instead of bright white for interactive prompts.
- For multi-select prompts, print a gray helper line before the prompt (for example: `Tip: press Space to toggle, Enter to confirm.`).
- For success output, color the action text in green (for example `Created todo`) and render metadata like `(id: ...)` in gray.

## Security Requirements

- Never print secrets or tokens in normal logs.
- Persist refresh/access tokens in OS keychain when possible.
- Keep non-secret metadata (selected account, URLs) in local config.
- Support token refresh without forcing re-login when refresh token is valid.

## Docs

Docs are available in:

- `AGENTS.md`
- `docs/agent/*`
