# Guardrails

## Rate limits

The Management API allows 60 requests per minute per key and 10 writes per minute. The CLI stays under these limits.

- **Limiter.** A token bucket shared by all CLI processes of your user, per env and key. The default is 0.8 requests per second and the hard cap is 1. Writes are spaced by 7 s. At most 4 requests are in flight. `--rate <n>` lowers the rate. A value above the cap exits 2, unless you add `--i-asked-backend`.
- **Circuit breaker.** On a 429 the CLI reads `Retry-After`, then `X-RateLimit-Reset`, then waits 60 s. Every process waits. The rate halves (down to 1/8) and recovers after 10 minutes without a 429. The request that got the 429 is retried once. After 20 rejections in 5 minutes the CLI stops with exit 7.
- **Usage log.** One line per request in `$XDG_STATE_HOME/purchasely/usage.jsonl` (default `~/.local/state/purchasely`), kept 24 hours. No key, no query value. `purchasely usage` shows the requests of the last hour, the rejections, and the limiter and breaker state.

## Writes

- `--env prod|staging` is mandatory on every write, also with `--dry-run`. Reads default to `prod` (or `PURCHASELY_ENV`).
- Every run prints one header line on stderr: `purchasely env=prod surface=management identity=- key=…a1b2`.
- `--reason "<why>"` goes to the server in the `User-Agent`: `purchasely-cli/<version> (-; <reason>; <command>; <runner>)`. The command is its name only (`placements list`), never the values you pass. The runner is `agent:<name>` when an AI agent runs it (`claude-code`, `codex`, `gemini-cli` or `cursor`, from the variable that the agent sets), `ci` (`CI` is set), `tty` (a terminal) or `script`. The CLI never sends an email.
- **Write allow-list.** List the apps that a write may change, per env, in `~/.config/purchasely/config.json`. A write to another app, or a write with no app, exits 10, also with `--dry-run`. An env with no list has no limit. An empty list refuses every write. Use it to keep an agent on a test app:

  ```json
  { "write_allowlist": { "prod": ["app_test"] } }
  ```

- **Read-only mode.** Set `PURCHASELY_READ_ONLY=1`, or put `"read_only": true` in `~/.config/purchasely/config.json`. The CLI then refuses every write before it sends it (exit 10). Reads and `--dry-run` still work. Use it for a script or an agent that only reads.

## Demo mode

`PURCHASELY_DEMO=1` turns on demo mode, for a first test by a person or an agent that has no key yet. `purchasely demo` prints how to start.

- Every read answers from the built-in sample data of one fictional app, `app_demo`: applications, placements, screens, products, plans (`--product-id prd_demo_premium`), campaigns and `dashboard summary`. Another read exits 6 with a message that demo mode has no data for it.
- The CLI uses no key, even when one is set, and sends no request: no limiter state, no usage line, no update check.
- A write exits 10 and sends nothing. `--dry-run` and `--plan` still work.
- The header line ends with `demo=on (sample data, not real)`. `doctor` reports demo mode instead of a missing key.

## Plan, then apply

An agent prepares the writes, a person reads them, then applies them.

- `--plan <file>` on a write records the request in a plan file and sends nothing. The checks of a write still run first: `--env`, `write_allowlist`. It works in read-only mode and with no key, so an agent can prepare a plan with `PURCHASELY_READ_ONLY=1`.
- `purchasely apply <file> --dry-run` lists the writes that are left. `purchasely apply <file>` lists them, asks once (or `--yes`), then sends them in order.
- Before each send, `apply` checks the entry again: the env is `prod` or `staging`, the method is a write, and the URL is the Management API of that env. Read-only mode and `write_allowlist` apply too. Any other entry exits 10, and the CLI sends nothing.
- `--yes` approves the plan, not a risky change. A risky change still asks in a terminal. Without a terminal, `apply` stops there (exit 8).
- Each applied write is marked (`applied_at`) in the file. `apply` stops at the first error (exit 9 when some writes were applied). Run it again to send the rest.

## Confirmation flow

A risky change (delete a used placement, replace webhooks) returns `CONFIRMATION_REQUIRED` with the impact and a `confirm_token`.

- `--yes`: the CLI prints the impact on stderr and resends the same request with the token, once. A second `CONFIRMATION_REQUIRED` exits 8.
- Interactive terminal without `--yes`: the CLI asks `Apply this change? [y/N]`.
- Not interactive without `--yes`: exit 8. The error JSON on stderr holds the token. Resend the same command with `--confirm-token <token>`.

The token goes where the spec says: in the query string for DELETE commands, in the JSON body for `webhooks update`, `plans create`, `translations create` and `audiences discard`.

## Secrets

Keys are never printed, logged or stored in a test snapshot. `--dry-run` shows `<redacted>`. See [limits.md](limits.md) for the keychain note.
