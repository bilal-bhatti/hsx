# hsx — project guidance

## Releasing

Releases are driven by `cargo release` (cargo-release 1.x), pinned via `.mise.toml`. Config lives in the `[package.metadata.release]` block in `Cargo.toml` (`publish = false`, `allow-branch = ["main"]`).

### Cutting a release

1. Commit all feature work first — `cargo release` requires a clean working tree and only runs on `main`.
2. `cargo release <patch|minor|major> --execute` (or `mise run release <level>`) — bumps `Cargo.toml` + `Cargo.lock`, commits `chore: Release hsx version X.Y.Z`, creates tag `vX.Y.Z`, and pushes `main` + the tag to `origin`.
3. Add `--no-confirm` for non-interactive runs (e.g. from an agent) — otherwise cargo-release blocks on a y/n prompt that can't be answered.

### What the tag push triggers

Pushing the `vX.Y.Z` tag fires two independent CI workflows:

- `.github/workflows/release.yml` — cargo-dist: builds binaries, the shell + Homebrew installers, pushes the formula to `bilal-bhatti/homebrew-taps`, and creates the GitHub Release (~5 min).
- `.github/workflows/cargo-publish.yml` — runs `cargo publish` to crates.io.

**crates.io publishing is CI's job, not local.** `publish = false` keeps local `cargo release` from publishing — do not publish locally, it collides with the CI workflow.

### Required secrets / setup (one-time)

The GitHub repo `bilal-bhatti/hsx` must exist with these repo secrets:

- `CARGO_REGISTRY_TOKEN` — crates.io API token (used by `cargo-publish.yml`).
- `HOMEBREW_TAP_TOKEN` — a PAT with write access to `bilal-bhatti/homebrew-taps` (used by `release.yml` to push the formula).

`GITHUB_TOKEN` is provided automatically.

### Gotcha: the "Publishing hsx" line

`cargo release` dry-run prints `Publishing hsx` even though `publish = false`. That line is cargo-release's generic release label, **not** the crates.io upload — a verbose dry-run (`--verbose`) shows zero packaging or upload activity. Don't be alarmed by it, and don't add `--no-publish` to "fix" it (harmless no-op; publish is already off).

### Versioning

Pre-1.0. Bump level is the maintainer's call per release — ask if unsure.
