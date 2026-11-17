# Development

The CLI is a Rust workspace. The toolchain version is in `.github/workflows/ci.yml` (`rust-version` in the crate manifests).

```bash
cargo build -p purchasely --bins
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p purchasely <module>::tests
```

The binary is `target/debug/purchasely`. The release build of the four targets is in `.github/workflows/build.yml`; `scripts/pack-release.sh` packs the archives (see [release.md](release.md)). The version is in `crates/purchasely/Cargo.toml`.

## Layout

- `crates/purchasely/src/main.rs`: the entry point. It calls `purchasely::main(Program { name, version, host, ext: None })`.
- `crates/purchasely/src/run.rs`: `build(rt)` and `run(argv, deps)`. Tests call `run()`.
- `crates/purchasely/src/core/`: limiter, breaker, credentials, http, output, usage, update, reference, AI plugin, and `real.rs` (the real host: files, network, keychain, processes).
- `crates/purchasely/src/management/`: the spec loader, the naming rules, the request building and the help of the generated commands.
- `crates/purchasely/src/commands/`: hand-written commands. `commands/mod.rs` tells how a module registers its commands.
- `crates/purchasely/src/cli/`: the command tree, the parser and the help.
- `crates/contract/`: dev only. The fake host, the replay of the recorded cases and the binary runner.
- `spec/management.json`: the Management OpenAPI spec, embedded in the binary. The commands come from it. Do not edit it by hand: it is a copy of the spec of the API.

## Extensions

A program built on `purchasely` passes an `Extension` (`crates/purchasely/src/core/ext.rs`) in `Program.ext`. It adds command modules after the built-in ones, the surface (credential and rate) of these commands, an optional identity in the `User-Agent`, a section in `reference`, and its own update channel. A new surface is added with `define_surface()` of `core/credentials.rs`. `purchasely` itself has no extension: it knows the `management` surface only.

## Registry rule

A hand-written command may take over a generated name: register it before the surface and add `resource action` to `rt.taken`.

## Tests

- Tests call `run(argv, deps)` with a fake `fetch`, keychain, clock and prompt. Do not call the network. Do not use real time in limiter tests.
- Unit tests live next to the code (`#[cfg(test)] mod tests`).
- `crates/contract/tests/replay.rs` replays `contract/cases/*.json` in process with the fake host of `crates/contract/src/fake.rs`. `acme_extension` of `crates/contract/src/replay.rs` is a small example extension for the tests of the extension seam.
- `crates/contract/tests/binary.rs` runs `target/debug/purchasely` (or `CONTRACT_BIN`) on `contract/bin/*.json` and compares stdout, stderr and the exit code. Build the binary first.
- `crates/contract/tests/install.rs` and `release.rs` run `install.sh` and `scripts/pack-release.sh` with fake tools on `PATH`. `docs.rs` checks the links and the content of the docs.
- `CONTRACT_FILTER=<file>::<test name prefix>` selects cases (a comma separates prefixes). A prefix that matches no case fails the run.
- The cases are frozen fixtures. To add or update one, write its inputs, then run with `CONTRACT_BLESS=1` and a filter that selects only that case. In `contract/cases`, first remove the outputs of a case that you update (`code`, `out`, `err`, `t_after`, `files_after`): the replay bless fails on an output that differs. Bless writes the outputs from the Rust run and marks the case `"source": "bless"`. Read the diff before you commit it.
- `docs/commands.md` is generated from the command tree: `scripts/gen-docs.sh`.
