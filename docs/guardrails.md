# Guardrails

## Rate limits

The Management API allows 60 requests per minute per key and 10 writes per minute. The CLI stays under these limits.

- **Limiter.** A token bucket shared by all CLI processes of your user, per env and key. The default is 0.8 requests per second and the hard cap is 1. Writes are spaced by 7 s. At most 4 requests are in flight. `--rate <n>` lowers the rate. A value above the cap exits 2, unless you add `--i-asked-backend`.
- **Circuit breaker.** On a 429 the CLI reads `Retry-After`, then `X-RateLimit-Reset`, then waits 60 s. Every process waits. The rate halves (down to 1/8) and recovers after 10 minutes without a 429. The request that got the 429 is retried once. After 20 rejections in 5 minutes the CLI stops with exit 7.
- **Usage log.** One line per request in `$XDG_STATE_HOME/purchasely/usage.jsonl` (default `~/.local/state/purchasely`), kept 24 hours. No key, no query value. `purchasely usage` shows the requests of the last hour, the rejections, and the limiter and breaker state.

## Writes

- `--env prod|staging` is mandatory on every write, also with `--dry-run`. Reads default to `prod` (or `PURCHASELY_ENV`).
- Every run prints one header line on stderr: `purchasely env=prod surface=management identity=- key=…a1b2`.
- `--reason "<why>"` goes to the server in the `User-Agent`: `purchasely-cli/<version> (-; <reason>)`. The CLI never sends an email.

## Confirmation flow

A risky change (delete a used placement, replace webhooks) returns `CONFIRMATION_REQUIRED` with the impact and a `confirm_token`.

- `--yes`: the CLI prints the impact on stderr and resends the same request with the token, once. A second `CONFIRMATION_REQUIRED` exits 8.
- Interactive terminal without `--yes`: the CLI asks `Apply this change? [y/N]`.
- Not interactive without `--yes`: exit 8. The error JSON on stderr holds the token. Resend the same command with `--confirm-token <token>`.

The token goes where the spec says: in the query string for DELETE commands, in the JSON body for `webhooks update`, `plans create`, `translations create` and `audiences discard`.

## Secrets

Keys are never printed, logged or stored in a test snapshot. `--dry-run` shows `<redacted>`. See [limits.md](limits.md) for the keychain note.
