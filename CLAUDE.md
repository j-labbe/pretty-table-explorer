# Project: pretty-table-explorer (pte)

Interactive terminal table viewer for PostgreSQL and piped data.

## Deployment Process

Releases are triggered by pushing a version tag. Follow these steps in order:

1. **Update version** in `Cargo.toml` and run `cargo check` to update `Cargo.lock`
2. **Commit and push** the version bump to `master`
3. **Create and push the tag**: `git tag v<VERSION> && git push origin v<VERSION>`
4. **Monitor CI** — the tag push triggers `.github/workflows/release.yml` which runs:
   - CI checks (fmt, clippy, tests) on ubuntu-latest
   - Cross-platform builds (linux-x86_64, linux-aarch64, macos-x86_64, macos-aarch64)
   - GitHub Release creation with binaries and checksums
5. **Wait for completion** before considering the release done:
   `gh run list --limit 1` or check the Actions tab on GitHub
