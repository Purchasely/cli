# Purchasely CLI

The `purchasely` command line tool for the Purchasely Management API. It reads and changes the apps, products, plans, placements, audiences, campaigns, flows and A/B tests of your account.

## Install

```bash
curl -fsSL https://cli.purchasely.io/install.sh | sh
```

You need Node 20 or newer. The installer puts the program in `~/.local/share/purchasely/install` and links `purchasely` into a bin directory that is already on your `PATH` (`~/.local/bin` or `~/bin`). If there is none, it creates `~/.local/bin` and adds one line to the profile of your login shell. It never uses sudo. Open a new terminal if the command is not found.

## Update

The CLI checks for a new version once a day and updates itself in the background. To update now:

```bash
purchasely update
```

To stop the daily check, set `PURCHASELY_DISABLE_AUTOUPDATE=1` or `PURCHASELY_OFFLINE=1`, or put `"auto_update": false` in `~/.config/purchasely/config.json`. The check also stays off when `CI` is set. Homebrew will come later.

## Use with your AI agent

```bash
purchasely ai install
```

This installs the Purchasely AI plugin in the coding agents it finds: Claude Code, Codex, Copilot CLI, Gemini CLI, Cursor and OpenCode. `--agent <name>` picks one. `--dry-run` shows the plan and changes nothing.

- `purchasely ai status` shows the agents, the plugin version and how each agent follows releases.
- `purchasely ai update` updates the plugin now. The CLI also updates it once a day in the background, only in the agents that `ai install` registered. A plugin you installed by hand is never touched.
- `purchasely ai uninstall` removes the plugin from the agents that `ai install` registered.

The plugin has 6 skills from plugin 2.3.0 (5 before): `purchasely-cli`, `sdk-expert`, `integrate`, `review`, `debug` and `migrate`. Ask your agent to integrate the SDK with the real placements, products, plans and custom events that the CLI reads from your account. Ask it to debug, review or improve your integration. The CLI does not return the SDK API key of an app: copy it from the Console (App settings) into your app configuration. The agent does not ask for it.

## Quick start

```bash
purchasely auth login   # asks for your Management API key
purchasely search "placement"                   # find a command by keyword
purchasely --help                               # every command
```

Read-only examples:

```bash
purchasely screens list --app app_xxx
purchasely placements get --app app_xxx --id <placement>
purchasely dashboard summary --app app_xxx --output table   # key metrics with charts
```

Add `--dry-run` to any command to print the request without sending it. A write needs `--env prod` or `--env staging`.

## Commands

Every Management API operation is a command: `purchasely <resource> <action>`. Run `purchasely --help`, then `purchasely <resource> --help`. `purchasely dashboard summary --app app_xxx --output table` shows the MRR, active subscriptions, paywall viewers and conversions of an app, with a chart per metric. `purchasely dashboard` alone opens a menu of every dashboard in a terminal, and prints the list as JSON with `--output json`. `purchasely usage` shows your requests of the last hour, the rejections and the limiter state.

`purchasely reference` prints every command (also `--format man|html|json`, and the page https://cli.purchasely.io/reference). `purchasely api GET /applications` calls any Management endpoint with the same rules. `purchasely completion zsh` prints a completion script (bash and fish too). `purchasely games` opens six terminal games.

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Generic failure (API error, network) |
| 2 | Wrong usage |
| 3 | No key, or the key is refused (401) |
| 4 | Forbidden (403) |
| 5 | Validation failed (422) |
| 6 | Not found (404) |
| 7 | Rate limited (429) |
| 8 | Confirmation required |
| 10 | Request refused by a local guardrail |

## Uninstall

Remove the install folder, the man page and the `purchasely` link in your bin dir (`command -v purchasely` shows it):

```bash
rm -rf ~/.local/share/purchasely/install
rm -f "${XDG_DATA_HOME:-$HOME/.local/share}/man/man1/purchasely.1"
rm "$(command -v purchasely)"
```

## Documentation

- [docs/install.md](docs/install.md): the installer, the update and the uninstall in detail.
- [docs/commands.md](docs/commands.md): every command and its help.
- [docs/guardrails.md](docs/guardrails.md): rate limits, writes and the confirmation flow.
- [docs/exit-codes.md](docs/exit-codes.md), [docs/limits.md](docs/limits.md).
- [docs/development.md](docs/development.md), [docs/release.md](docs/release.md), [AGENTS.md](AGENTS.md): for contributors and coding agents.

## License

Copyright (c) 2026 Purchasely SAS. All rights reserved. Only customers of Purchasely may use the CLI, to use the Purchasely platform. See [LICENSE](LICENSE).
