# AGENTS.md

Rules for an AI coding agent that uses or changes the Purchasely CLI (`purchasely`). Do not create a `CLAUDE.md`. This file is the only agent file.

## Use the CLI

1. Find a command: `purchasely search "<task>"`. It returns a JSON array. The first item is the best match.
2. Read the help of that one command only: `purchasely <resource> <action> --help`.
3. Read stdout (JSON) and the exit code. Do not parse stderr, except for the error JSON.
4. Run `--dry-run` before every write. Show the request to the user.
5. A write needs `--env prod` or `--env staging`. Use `staging` unless the user says `prod`.
6. Do not pass `--yes` without the approval of the user. First show the impact (the `error.messages`).
7. For a confirmation, rerun the same command with `--confirm-token <token>` after the user approves. The token is in the stderr JSON. It lives 5 minutes.
8. Do not print the API key. Do not put it in a command. Use `PURCHASELY_API_KEY` or `purchasely auth login`.
9. Do not raise `--rate` and do not use `--i-asked-backend`. The limiter protects the server.
10. If you get exit 7, stop. Wait and read `purchasely usage`. Do not loop.
11. Add `--reason "<why>"` on every command. It goes to the server in the User-Agent.
12. Large answers go to a file (`{file, bytes}`). Read the file. Use `--fields` to ask for less.
13. Do not run `update` or `install.sh` unless the user asks.

Examples: `purchasely screens list --app app_xxx`, `purchasely placements get --app app_xxx --id <placement>`, `purchasely dashboard summary --app app_xxx` (key metrics; add `--output table` for charts when a person reads it).

## Which command for which question

| Question | Command |
|---|---|
| The key metrics of one app | `purchasely dashboard summary --app <app_id>` |
| The list of the dashboards | `purchasely dashboard --output json` (`command`, `description`, `required_flags`). Do not open the menu: it needs a terminal |
| A Management endpoint with no generated command | `purchasely api <METHOD> <path>` (same rules: `--dry-run`, `--env` on a write, the confirmation) |
| Every command, for an agent | `purchasely reference --format json` |
| Any other task | `purchasely search "<task>"` |

The CLI gives no screen JSON. Do not look for a way to get it.

## Exit codes

See [docs/exit-codes.md](docs/exit-codes.md). The short rule: 7 means stop, 9 means the data is not complete, 10 means a write was refused.

## Change the CLI

- The commands come from `spec/management.json` at runtime. Do not edit that file by hand.
- Layout and rules: [docs/development.md](docs/development.md).
- `docs/commands.md` is generated from the command tree. Change the help text of the command, not the file, then run `npx vitest run test/docs.test.ts -t "commands.md is generated" -u`.
- A hand-written command may take over a generated name: register it before the surface and add `resource action` to `rt.taken`.
- Tests call `run(argv, deps)` with a fake `fetch`, keychain, clock and prompt. Do not call the network. Do not use real time in limiter tests.
- Commits: atomic. Use Conventional Commits. The full suite runs in CI: locally run `npm run typecheck` and the tests of your change.
- Do not use em-dashes or en-dashes in docs or help text.
- Never write the key in a log, an error or a test snapshot.
