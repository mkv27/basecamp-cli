# Code Plan (Auth Feature)

## Selected OAuth Dependency

Use:

```toml
oauth2 = { version = "5", default-features = false, features = ["reqwest", "rustls-tls"] }
```

Decision notes:

- There is no Basecamp-official Rust SDK for OAuth; Basecamp documents OAuth flow and recommends using an OAuth2 library.
- `oauth2` (oauth2-rs) is the de facto Rust OAuth2 crate and is listed in OAuth.net's Rust libraries.
- It directly supports the required flow in `cli.md`:
  - authorization URL creation
  - authorization code exchange
  - refresh token exchange
  - CSRF state handling

## Why This Fits `cli.md`

`docs/agent/features/auth/cli.md` requires:

1. `integration set/show/clear` for `client_id`, `client_secret`, `redirect_uri`
2. `login` with local callback + code exchange + refresh support
3. `logout` clearing local session state

`oauth2` maps cleanly to login internals:

- `Basecamp OAuth URL` -> `authorize_url(...)`
- `callback code` -> `exchange_code(...)`
- `token refresh` -> `exchange_refresh_token(...)`
- `state validation` -> generated CSRF token compare

The callback listener itself is CLI runtime logic (not provided by `oauth2`), which matches our local loopback-server model.

## Proposed Auth Module Layout

- `src/features/auth/integration.rs`
  - read/write `client_id`, `redirect_uri`
  - store/remove `client_secret` in secret store
- `src/features/auth/oauth.rs`
  - `oauth2::basic::BasicClient` setup
  - auth URL building, token exchange, refresh
- `src/features/auth/callback.rs`
  - short-lived local callback server for `code` + `state`
- `src/features/auth/login.rs`
  - orchestration flow from `integration` + `oauth` + `callback`
- `src/features/auth/logout.rs`
  - clear tokens/account session data

## Dependency Governance

- This document records the selected OAuth dependency.
- Per `AGENTS.md`, dependencies are only added to `Cargo.toml` after explicit user approval.
- If dependency details change (version/features), update this file and get re-approval.

## References

- Basecamp auth docs: <https://github.com/basecamp/bc3-api/blob/master/sections/authentication.md>
- oauth2-rs docs: <https://docs.rs/oauth2/latest/oauth2/>
- OAuth.net Rust libraries: <https://oauth.net/code/rust/>
