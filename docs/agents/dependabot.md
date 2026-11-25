# Dependabot

What reviewing a Dependabot pull request takes here, read by the `dependabot` skill in a session
and in the scheduled review. These facts apply to this repository only.

## Stack

- **The Rust workspace** (`Cargo.toml`, `Cargo.lock`, `crates/*`, `apps/*`): sim-racing overlays,
  Windows-only. `windows` maps iRacing's shared memory and opens its data-valid event
  (`crates/irsdk/src/windows.rs`); `encoding_rs` decodes iRacing's Windows-1252 strings;
  `serde_yaml_ng` parses the session YAML (`crates/iracing/src/session.rs`); `thiserror` and
  `anyhow` carry the errors; `clap` parses the `reo` CLI.
- **Toolchain:** `rust-toolchain.toml` pins Rust 1.100, the same as `rust-version` in Cargo.toml
  (the MSRV). Dependabot does not bump it.
- **Node 26** (`.nvmrc`) runs the repository scripts, which have no dependencies and no
  `package.json`.

`.github/workflows/ci.yml` runs three jobs, all required on `main`: `rust` (on `windows-latest`:
`rustup toolchain install`, then `cargo fmt --all --check`, `cargo clippy --workspace
--all-targets --locked -- -D warnings` and `cargo test --workspace --locked`), `changelog`
(`node scripts/check-changelog.mjs`) and `agents` (the repository scripts on Node 26).
`release.yml` runs on a push to `main`: `tag` stamps the changelog and tags the merge, and
`package` builds the release zip on Windows.

`.github/dependabot.yml` opens updates daily; one review covers a grouped PR:

- **cargo, `/`:** `cargo-minor-and-patch` groups minor and patch updates; each major is its own PR.
  The `windows`/`windows-*` crates travel together in `windows-crates`, majors included.
- **github-actions, `/`:** `actions-minor-and-patch`; each major is its own PR. Actions are pinned
  by commit SHA with a version comment, and Dependabot updates both.

## Review triggers

- **`windows`, on any bump.** Its `0.x` minors are breaking: feature names, handle types and
  function signatures move (`OpenFileMappingW`, `MapViewOfFile`, `VirtualQuery`, `OpenEventW`,
  `WaitForSingleObject` in `crates/irsdk/src/windows.rs`). CI builds that code, but no test opens
  a real iRacing mapping.
- **`serde_yaml_ng`, on any bump.** iRacing's session YAML is not always valid YAML, and the
  parser's leniency is what `reo_iracing::session::sanitize` relies on; a change in what it
  accepts reaches every driver name.
- **A dependency whose new version raises its own `rust-version` above 1.100.** CI builds on 1.100,
  so it goes red; that is the MSRV holding, not a flake (Grounding facts).

## Resolution

- **Toolchain.** `rustup toolchain install` at the repository root installs the 1.100 toolchain
  `rust-toolchain.toml` names, with clippy and rustfmt. Node 26 from `.nvmrc`.
- **cargo** (`Cargo.toml`, `Cargo.lock`), from the repository root:
  `cargo build --workspace --locked`.

## Where to look

- `Cargo.toml`'s `[workspace.dependencies]` (versions and the `windows` feature list) and
  `[workspace.lints]`.
- `crates/irsdk/src/windows.rs` (windows), `crates/irsdk/src/layout.rs` (encoding_rs),
  `crates/iracing/src/session.rs` (serde_yaml_ng), `apps/reo-cli/src/main.rs` (clap, anyhow).
- The `with:` inputs in `ci.yml` and `release.yml`.

## Grounding facts

- **CI builds on Rust 1.100, the MSRV.** A dependency that needs a newer compiler fails the `rust`
  job with an error naming the required `rustc` version. The fix is a decision, not a retry: hold
  the dependency back, or raise `rust-version` and `rust-toolchain.toml` together in their own PR.
- **No test touches a running sim.** The suite builds synthetic shared-memory images
  (`reo_irsdk::testing::ImageBuilder`); a green CI says nothing about a live iRacing mapping. The
  maintainer can replay a recorded race locally with `reo check <tape>` (`AGENTS.md`).
- **The changelog check says nothing about the dependency.** CI runs `check-changelog.mjs` with no
  `--source`, so a branch with no `changelog.d/` fragment, as a Dependabot branch is, passes it. At
  release, `scripts/stamp-changelog.mjs` writes that merge's entry under `Changed` from its PR title.
- **Clippy warnings are errors.** A clippy that comes with a toolchain change can add lints; a
  dependency bump alone does not change clippy, since the toolchain is pinned.

## Playbook

- **`windows` minor (0.x):** read its release notes, then search `crates/irsdk/src/windows.rs` for
  each changed API and check the feature list in `Cargo.toml`. Gates: the Resolution build,
  `cargo clippy` and `cargo test --workspace --locked`. Ask the maintainer to run `reo watch`
  against a live iRacing session before merging.
- **GitHub Actions major:** NEEDS EFFORT only when a removed or renamed input, or a changed default,
  reaches a `with:` input; name the step. The inputs in use: `actions/checkout` (`fetch-depth: 0`,
  `ref`, `token`, `persist-credentials: false`), `actions/setup-node` (`node-version-file: .nvmrc`),
  `actions/create-github-app-token` (`client-id`, `private-key`, `permission-contents: write`) and
  `Swatinem/rust-cache` (none).
