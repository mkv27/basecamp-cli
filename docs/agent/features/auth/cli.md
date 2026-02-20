# CLI Contract (Auth Feature)

This feature defines all authentication commands and credential/session behavior.

## Authentication Model

- Basecamp API access is OAuth 2.0 based.
- This CLI uses OAuth Authorization Code flow.
- No non-OAuth auth mode is supported.

## Integration Requirement (`client_secret` caveat)

- OAuth requires `client_id` and `client_secret` from a Basecamp integration.
- Each user should create and use their own integration in Basecamp Launchpad.
- Do not embed or distribute one shared `client_secret` for all users.

## Command Surface

```bash
basecamp-cli integration set --client-id <id> --client-secret <secret> --redirect-uri <uri>
basecamp-cli integration show
basecamp-cli integration clear [--force]
basecamp-cli login [--account-id <id>] [--no-browser] [--json]
basecamp-cli logout [--forget-client] [--json]
```

## Command Details

### `basecamp-cli integration set`

Purpose:
- Store OAuth integration credentials used by `basecamp-cli login`.

Required flags:
- `--client-id <id>`
- `--client-secret <secret>`
- `--redirect-uri <uri>`

Behavior:
1. Validate input values.
2. Persist `client_secret` in secure secret storage.
3. Persist non-secret client config (`client_id`, `redirect_uri`) in local config.
4. Print confirmation without exposing secrets.

### `basecamp-cli integration show`

Purpose:
- Display current auth configuration status.

Behavior:
1. Show whether `client_id`, `client_secret`, and `redirect_uri` are configured.
2. Never print raw `client_secret`.
3. Optionally show redacted `client_id` for debugging.

### `basecamp-cli integration clear`

Purpose:
- Remove saved OAuth integration credentials.

Behavior:
1. Require confirmation unless `--force` is passed.
2. Delete stored `client_secret`.
3. Delete stored `client_id` and `redirect_uri`.
4. Keep or remove token session based on implementation policy, but document it in help text.

### `basecamp-cli login`

Purpose:
- Authenticate user account and establish a reusable Basecamp session.

Inputs precedence:
1. Explicit flags
2. Environment variables
3. Stored config from `basecamp-cli integration set`

Supported env names:
- `BASECAMP_CLIENT_ID`
- `BASECAMP_CLIENT_SECRET`
- `BASECAMP_REDIRECT_URI`

Callback model:
- CLI starts a short-lived local loopback callback server.
- Browser redirects to configured `redirect_uri`.
- CLI captures OAuth `code` and completes token exchange in-process.
- No permanent standalone backend service is required.

Behavior:
1. Resolve client config from precedence order.
2. Validate required values.
3. Generate CSRF `state`.
4. Start temporary local callback listener.
5. Open browser to OAuth authorization URL (or print URL with `--no-browser`).
6. Receive callback, verify `state`, extract `code`.
7. Exchange `code` for `access_token` and `refresh_token`.
8. Call `authorization.json`.
9. Filter accounts where `product == "bc3"`.
10. Auto-select single account, or prompt if multiple.
11. Persist session tokens and selected account metadata.
12. Stop callback listener and return success.

Optional flags:
- `--account-id <id>`
- `--no-browser`
- `--json`

### `basecamp-cli logout`

Purpose:
- End local authenticated session.

Behavior:
1. Remove stored access/refresh tokens.
2. Remove selected account metadata from current session profile.
3. Keep OAuth client config by default.
4. If `--forget-client` is set, also clear integration credentials (equivalent to `integration clear --force`).

Optional flags:
- `--forget-client`
- `--json`

## Output

Human output example:

```text
Logged in to Basecamp account "Acme Co" (123456789).
```

JSON output example:

```json
{
  "ok": true,
  "account_id": 123456789,
  "account_name": "Acme Co"
}
```

## Exit Codes

- `0`: success
- `1`: generic failure
- `2`: invalid CLI/config input
- `3`: OAuth callback or token exchange failure
- `4`: no accessible `bc3` account found
- `5`: secure storage read/write failure

## Persistence Model

Secrets:
- `client_secret`
- `access_token`
- `refresh_token`

Non-secret config/session:
- `client_id`
- `redirect_uri`
- `account_id`
- `account_name`
- `account_href`
- `updated_at`

Storage rules:
- Store secrets in OS keychain if available.
- Store non-secrets in local config file.
- Never print secrets in logs or standard command output.

## Non-Goals (Auth Feature)

- Listing projects, todos, or people
- Multi-profile account switching commands
- Hosted standalone auth backend service
