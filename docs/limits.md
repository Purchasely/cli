# Limits

## Product

- The public host holds the latest version only. `install.sh --version <old>` fails on the public host.
- You need Node 22.12 or newer. There is no standalone binary yet.
- No Windows. There is no installer and no launcher for it.
- fish users add `~/.local/bin` to PATH by hand. The installer writes `~/.profile` for any shell that is not zsh or bash.
- The CLI gives no screen JSON. The Management API omits the `content` of a screen for a client key.

## AI plugin (`ai`)

- Gemini CLI: `ai install` works. `ai update` and `ai uninstall` always refuse and print the native command (`gemini extensions update purchasely`). The Gemini list shows the name only, so the CLI cannot prove it owns the plugin.
- The agent CLIs run with a 120 s timeout.
- Claude Code needs a restart to load an updated plugin. Cursor needs a reload.
- The plugin tarball has no checksum. It comes over HTTPS from the public plugin repo (latest release).
- Native agents follow the default branch of the plugin repo, not a release. `ai status` says so.
- The SDK API key of an app is not available from the CLI. Copy it from the Console (App settings).

## Data

- Management `screens list` returns one page.
- There are no per-field body flags. Send the body with `--data`.
- Command names are derived from the spec by rules, not by the API team.

## Security

- On macOS the CLI stores a key with the `security` CLI. The key goes to it on stdin, never on its argv, so another process cannot read it there.
- The keychain read at each run also uses `security`. The secret goes to the CLI through stdout, not argv.
