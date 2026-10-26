# Install and update

The installer is `install.sh`. Its line `HOST=` is set by `scripts/pack-release.sh` to the public host.

- Bootstrap: `curl -fsSL https://cli.purchasely.io/install.sh | sh`
- Prerequisites: Node 20 or newer, `curl`, `tar` and `gzip`.
- Install root: `${XDG_DATA_HOME:-~/.local/share}/purchasely/install`.
- Versions: the latest only (the site holds one snapshot).

`PURCHASELY_INSTALL_ROOT` or `--root <dir>` changes the root. The root must be an absolute path without a single quote.

## Layout of a root

```
<root>/releases/<version>/   one directory per version (dist/, spec/, package.json, ...)
<root>/current-version       the active version
<root>/bin/<name>            launcher: sets PURCHASELY_MANAGED_INSTALL_ROOT, runs node on the active release
<root>/staging/              download area and lock
```

The PATH entry is a symlink `<bindir>/<name>` to `<root>/bin/<name>`.

## Bin dir and profile rule

1. `PURCHASELY_BIN_DIR` is set: use it. The installer edits no profile. It prints a hint when the directory is not on PATH.
2. Else the first of `~/.local/bin` and `~/bin` that exists, is writable and is already on PATH. No profile edit.
3. Else create `~/.local/bin` and add one line, guarded by the marker `# added by the purchasely installer`, to the profile of the login shell: `~/.zshrc` for zsh, `~/.bashrc` and `~/.bash_profile` for bash, `~/.profile` for anything else (fish included). A file that has the marker is never changed again.

The installer never uses `sudo`. After a profile edit it prints `open a new terminal if the command is not found`. It does not replace a real file at the link path, and it warns when another `<name>` comes first on PATH (an old npm install). `--upgrade-only` changes no link and no profile.

## Man page

Each release holds `man/<name>.1`, made by `<name> reference --format man`. After the switch, `install.sh` copies it to `${XDG_DATA_HOME:-$HOME/.local/share}/man/man1/<name>.1` (a temporary file, then `mv`). It does this on every successful install: a fresh install, the same version again (this repairs a missing page), an update and `--upgrade-only`. A release without a page changes nothing. A failed copy prints one warning and does not fail the install. An install of an older release without a page leaves the newer page in place.

A fresh install and a repeat install (not `--upgrade-only`) check that `man` finds the page. When `man` exists, the installer runs `man -w <name>` with the bin dir first on PATH, and prints `To read "man <name>", add to your shell profile: export MANPATH="<mandir>:$MANPATH"` only when `man` does not find the installed page. `man` adds `<dir without /bin>/share/man` for each PATH entry that ends in `/bin`, so the default layout (`~/.local/bin` and `~/.local/share`) needs no hint. When `man` is missing, the installer applies the same rule to the bin dir: an explicit man dir in `MANPATH` counts; a `MANPATH` with no empty component is the whole search path; an unset `MANPATH`, or one with an empty component, adds the dirs derived from PATH.

## `install.sh` arguments and exits

`install.sh [--version <ver>] [--keep <ver>] [--log <file>] [--root <dir>] [--upgrade-only]`. `PURCHASELY_VERSION` also sets the version.

| Exit | Meaning |
|---|---|
| 0 | Installed, or already up to date |
| 1 | Download, checksum or smoke test failure |
| 2 | Prerequisite missing, or a bad argument |
| 3 | Another install holds the lock |

The script prints fixed messages only. It never prints the output of `curl`. The release is verified against `SHA256SUMS` and smoke tested (`--version`) before it becomes the active version.

## Site files (public host)

The Cloudflare Pages project `purchasely-cli` serves: the landing page, `install.sh`, `latest.json` (`{"version","sha256"}`), `purchasely-<ver>.tgz`, `SHA256SUMS` and `_headers`. The host value lives in `package.json` `config.host` only.

## Update mechanics

- Install kind: `managed` when `PURCHASELY_MANAGED_INSTALL_ROOT` is set (the launcher sets it), `brew` when the module path holds `/Cellar/`, else `npm`.
- The first command of each day (the record `update.<name>.check.json` in the state dir) starts a detached `update --background`. The parent never waits. For a managed install the child runs `install.sh --upgrade-only`. The log is `update.<name>.log`.
- Notices (stderr, once a day, only when the record holds a newer version): managed, none; brew, `brew upgrade purchasely`; npm, `curl -fsSL https://cli.purchasely.io/install.sh | sh`.
- Off: `PURCHASELY_DISABLE_AUTOUPDATE=1`, `PURCHASELY_OFFLINE=1`, `CI` set, or `"auto_update": false` in `~/.config/purchasely/config.json`.
- `update` checks now and, for a managed install, installs. For other kinds it prints the upgrade command.

## Plugin update mechanics

`purchasely ai install` registers the Purchasely AI plugin in a coding agent and writes the record `ai.plugin.json` in the state dir. Only the CLI writes it.

- Once a day (daily): the detached `update --background` child runs the plugin step after the CLI step, in its own `try/catch`, with its own `checked_at`. The parent starts the child when the CLI check or the plugin check is due. Only the agents in the record are updated.
- Ownership: before every update and uninstall the CLI probes the agent again and compares the recorded identity. It refuses when the plugin is replaced, ambiguous or cannot be probed. The CLI never touches a plugin that you installed by hand.
- Native agents (Claude Code, Codex, Copilot CLI, Gemini CLI) run their own plugin commands. Cursor and OpenCode get a copy of the files of the latest plugin release.
- Off: the same opt-outs as the CLI update (`PURCHASELY_DISABLE_AUTOUPDATE=1`, `PURCHASELY_OFFLINE=1`, `CI`, `"auto_update": false`). `ai install` and `ai update` also stop under `PURCHASELY_OFFLINE=1`, because the agent CLIs use the network.

## Paths

Config `~/.config/purchasely`, state `~/.local/state/purchasely`, cache `~/.cache/purchasely`, and the keychain service `purchasely-cli`.

## Uninstall

- Remove `<bindir>/purchasely` and `~/.local/share/purchasely/install`.
- Delete by hand the profile line that ends with `# added by the purchasely installer`, if the installer added it.
- Optional: `purchasely auth logout`, and delete the config, state and cache directories.

## Troubleshooting

- `command not found`: open a new terminal. With fish, add `~/.local/bin` to PATH by hand.
- `Node 20 or newer is required`: install Node (`brew install node`).
- `only the latest version ... is published`: do not pass `--version`.
- `<name> exists but is incomplete`: remove that release directory and run again.
- An old npm copy comes first on PATH: uninstall it (`npm uninstall -g @purchasely/cli`).
