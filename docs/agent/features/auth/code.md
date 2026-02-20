# Code Plan (Auth Feature)

## Selected OAuth Dependency

Use:

```toml
oauth2 = { version = "5", default-features = false, features = ["reqwest", "rustls-tls"] }
```

Supporting runtime dependencies:

```toml
reqwest = { version = "0.12.24", default-features = false, features = ["json", "rustls-tls"] }
tokio = { version = "1.48.0", features = ["macros", "rt-multi-thread"] }
serde = { version = "1.0.228", features = ["derive"] }
serde_json = "1.0.145"
url = "2.5.7"
dialoguer = "0.12.0"
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

## `integration set` UX Upgrade

When `basecamp-cli integration set` runs with missing arguments, the command should switch to interactive input.

Prompt flow (in order):

1. `client_id` prompt (normal visible input)
2. `client_secret` prompt (hidden/no-echo password input)
3. `redirect_uri` prompt (visible input)

Rules:

- If a value is passed by flag, do not prompt that field.
- If running in non-interactive mode (no TTY) and any required value is missing, fail with exit code `2`.
- Never print the raw `client_secret`.
- Reuse existing stored values as defaults only for visible fields (`client_id`, `redirect_uri`), never for secret echo.

Implementation note:

- Keep it simple. Use a minimal hidden-input approach for password entry (for example, `rpassword`) if needed.

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

## `redirect_uri` Requirement

For this CLI, `redirect_uri` must be the exact callback URL registered in the user’s Basecamp integration.

Recommended value:

```text
http://127.0.0.1:45455/callback
```

Required properties:

- exact match with Basecamp integration settings
- `http` scheme for local loopback callback
- host must be `127.0.0.1` or `localhost`
- explicit port is required
- stable callback path (recommended `/callback`)

If user chooses another port/path, they must update both:

1. Basecamp integration `redirect_uri`
2. local CLI integration config (`basecamp-cli integration set`)

## Secret Storage Strategy (simple, codex-rs inspired)

Model:

- Use OS keyring as the main secret-key backend.
- Store secret values in an encrypted local file.
- Encryption/decryption key is generated locally and stored in keyring.
- Keep implementation simple:
  - one concrete storage path
  - no pluggable backend system for now
  - no extra abstraction beyond what auth commands need

Linux + macOS + Windows behavior:

- macOS: keyring backend uses Apple Keychain.
- Linux: keyring backend uses native Secret Service integration.
- Windows: keyring backend uses Windows Credential Manager (`windows-native`).
- Secret operations require keyring availability.

No runtime fallback:

- If keyring load/save fails, secret operations fail.
- This matches codex-rs behavior (`set_fails_when_keyring_is_unavailable` test).

Project path plan:

- Secret file path should be under app config root:
  - `${BASECAMP_CLI_CONFIG_DIR}/secrets/local.age`, or
  - `${XDG_CONFIG_HOME:-~/.config}/basecamp-cli/secrets/local.age`
  - Windows default: `%APPDATA%\\basecamp-cli\\secrets\\local.age` (fallback `%LOCALAPPDATA%\\basecamp-cli\\secrets\\local.age`)
- Non-secrets remain in JSON config file.
- Unix permissions:
  - secret dir `0700`
  - secret file `0600`
- Windows permissions:
  - rely on user profile ACL defaults for `%APPDATA%`/`%LOCALAPPDATA%`
  - no Unix mode bits are applied on Windows

## Storage Location Logging (Required)

Every command that touches secrets must print where data is stored, in gray.

- For keyring:
  - print keyring service + account identifier
  - example: `using secret store: keyring service=basecamp-cli account=secrets|abc123`
- For encrypted local file:
  - print absolute file path
  - example: `using secret file: /home/user/.config/basecamp-cli/secrets/local.age`

CLI rendering rule:

- Use `colored` and render storage-location lines with `.bright_black()`.
- Do not print secret values.

## Dependency Governance

- This document records the selected OAuth dependency.
- Per `AGENTS.md`, dependencies are only added to `Cargo.toml` after explicit user approval.
- If dependency details change (version/features), update this file and get re-approval.

## Candidate Secret-Store Dependency Set

Approved for this project:

```toml
keyring = { version = "3.6.3", default-features = false, features = ["crypto-rust"] }
age = "0.11.2"
rand = "0.10.0"
sha2 = "0.10.9"
base64 = "0.22.1"
```

Target-specific features (codex-rs aligned):

- Linux: `linux-native-async-persistent`
- macOS: `apple-native`
- Windows: `windows-native`

## Testing Strategy (Simple)

Keep auth testing minimal and predictable:

1. Unit tests (fast, no network)
   - validate `integration set/show/clear` behavior
   - validate OAuth URL/state construction
   - validate callback parsing and `state` mismatch handling
2. Mocked HTTP tests (no real Basecamp)
   - mock token exchange and `authorization.json`
   - verify request shape and response handling
3. Manual smoke test (real integration, local run)
   - run `integration set`
   - run `login`
   - run `logout`

Notes:

- CI should run only unit + mocked HTTP tests.
- Real Basecamp OAuth tests should remain manual (or gated/ignored), because they require user credentials and browser interaction.

## References

- Basecamp auth docs: <https://github.com/basecamp/bc3-api/blob/master/sections/authentication.md>
- oauth2-rs docs: <https://docs.rs/oauth2/latest/oauth2/>
- OAuth.net Rust libraries: <https://oauth.net/code/rust/>
- codex-rs keyring deps: <https://github.com/openai/codex/blob/main/codex-rs/keyring-store/Cargo.toml>
- codex-rs local secrets backend: <https://github.com/openai/codex/blob/main/codex-rs/secrets/src/local.rs>
