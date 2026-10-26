# Security

## Report a vulnerability

Do not open a public issue for a vulnerability. Use the private report of GitHub: open the Security tab of this repository, then click Report a vulnerability.

Do not put a real API key in the report. Give the version (`purchasely --version`) and the steps to reproduce.

## Supported versions

The latest release only. The CLI updates itself, and `purchasely update` installs the latest version.

## Check a download

Each release has a provenance that GitHub signs. Check the tarball and the installer before you run them:

```sh
gh attestation verify purchasely-<version>.tgz -R Purchasely/cli
gh attestation verify install.sh -R Purchasely/cli
```

`install.sh` also checks the tarball against `SHA256SUMS` before it installs it.

## What the CLI does to protect your account

See [docs/guardrails.md](docs/guardrails.md): the rate limiter, `--dry-run`, the confirmation of a risky change, read-only mode, the write allow-list, plan then apply, and the key that is never printed.
