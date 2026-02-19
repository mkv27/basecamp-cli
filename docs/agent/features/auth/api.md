# Basecamp API (Auth Feature)

This project uses Basecamp 3/4 API via OAuth 2.0 through 37signals Launchpad.

Reference: <https://github.com/basecamp/bc3-api>

## Core Concepts

- Authentication is OAuth 2.0 Authorization Code flow.
- API responses are JSON.
- After login, account access is discovered from `authorization.json`.
- Basecamp account APIs are accessed through account-specific base URLs.

## Command to API Mapping

- `basecamp integration set/show/clear` are local credential/config operations (no Basecamp API call).
- `basecamp login` performs OAuth authorization + token exchange + `authorization.json` discovery.
- `basecamp logout` is local session/token removal (no Basecamp API logout endpoint required).

## OAuth Endpoints

Authorization URL:

```text
https://launchpad.37signals.com/authorization/new
```

Token URL:

```text
https://launchpad.37signals.com/authorization/token
```

Authorization summary URL:

```text
https://launchpad.37signals.com/authorization.json
```

## Login Flow

1. Build the authorization URL with:
   - `response_type=code`
   - `client_id=<client_id>`
   - `redirect_uri=<redirect_uri>`
   - `state=<csrf_token>`
2. User authorizes in browser.
3. Redirect callback receives `code` and `state`.
4. Exchange `code` for tokens at token endpoint with:
   - `grant_type=authorization_code`
   - `client_id`
   - `redirect_uri`
   - `client_secret`
   - `code`
5. Use `Authorization: Bearer <access_token>` to call `authorization.json`.
6. From returned `accounts`, select entries where `product == "bc3"`.
7. Persist selected account info and tokens for future API calls.

## Refresh Flow

When access token expires, request a new one with:

- `grant_type=refresh_token`
- `refresh_token=<refresh_token>`
- `client_id`
- `client_secret`

Legacy `type=web_server` and `type=refresh` are still accepted, but new code should prefer standard OAuth parameters.

## Request Rules

- Always send `User-Agent` with app name/contact.
- Send `Authorization: Bearer <access_token>` for authenticated requests.
- Treat `401` as expired/invalid token and attempt refresh once.
- Treat `403` as permission denial.
- Handle `429` with retry/backoff.

## Data Needed After Login

- `access_token`
- `refresh_token`
- selected `account.id`
- selected `account.href`
- selected `account.name`
- timestamp for `updated_at`
