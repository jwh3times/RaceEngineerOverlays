# AGENTS.md

Guidance for coding agents (and humans) working in this repository. It holds what applies to
**every** change. Area detail lives in `docs/`, linked below. Read the relevant one before
changing that area.

## What this is

Race Engineer Overlays: sim-racing overlays built from scratch for performance, with a Rust core,
native Direct2D overlays and a web-based settings app. iRacing comes first, with more sims to
follow. "reo" is the working name used in crate and binary names. Licensed AGPL-3.0-only. Don't
add dependencies with incompatible licenses (MIT, Apache-2.0, BSD, MPL-2.0 and LGPL are fine).

Read [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) before structural changes. It records the
stack decision, the crate roles, the measured numbers and the roadmap.

## Commands

```bash
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all                             # CI runs `cargo fmt --all --check`
cargo build --release --locked -p reo-cli

# Validate the core against a recorded race (no sim needed)
target/release/reo check irdashies/test-data/telemetry/ai-race-10min.irdt
target/release/reo watch --irdt irdashies/test-data/telemetry/ai-race-10min.irdt --speed 10
target/release/reo vars  --irdt <tape>      # every variable with its current value
target/release/reo session --raw --irdt <tape>

# Live (iRacing running)
target/release/reo watch
```

`rust-toolchain.toml` pins the compiler (Rust 1.100, the same as `rust-version` in `Cargo.toml`);
rustup installs it on the first `cargo` run.

The repository scripts need only Node 26 (`.nvmrc`), with nothing to install. CI's `agents` and
`changelog` jobs run these:

```bash
node scripts/sync-agents.mjs --check     # agent config is in sync
node scripts/check-skills.mjs            # installed skills match skills-lock.json
node scripts/lint-agent-neutrality.mjs   # skill and agent sources read correctly from both trees
node scripts/check-docs.mjs              # doc links resolve and docs/agents/docs.md maps every doc
node scripts/check-cargo-version.mjs     # Cargo.toml's workspace version matches VERSION
node scripts/check-changelog.mjs         # the branch's changelog.d/ fragment is valid
node --test "scripts/*.test.mjs"         # tests for the scripts
```

These two preview what the next release will do, and only run by hand. After the merge,
`release.yml` runs `stamp-changelog.mjs` for real, without `--dry-run`:

```bash
node scripts/next-version.mjs            # the version the next merge to main will mint
node scripts/stamp-changelog.mjs --dry-run   # the sections the release workflow would stamp
```

Once a PR is open, read [docs/agents/merging.md](docs/agents/merging.md).

## Layout

- `crates/model`: the sim-agnostic model. The only contract between sim adapters and everything
  downstream. Adapters must not leak sim-specific types past it.
- `crates/irsdk`: iRacing shared-memory reader.
- `crates/iracing`: iRacing adapter onto the model.
- `apps/reo-cli`: developer CLI (`reo`).
- `docs/`: architecture and decisions; `docs/agents/` holds the settings the shared skills read.
- `scripts/`: repository scripts (Node, no dependencies) and their tests.
- `irdashies/`: an unmodified clone of upstream irDashies (MIT), kept for **reference only**. It
  is not part of this project's build and is ignored by git. Don't copy its code; its test-data
  tapes are used for local validation.

## Rules for every change

1. **Model units are SI** (m, s, L, rad, m/s). Pedals and fractions are `0.0..=1.0`. Anything a
   sim doesn't provide is `None`, never a sentinel. Translate sim sentinels (`-1`, `0`, `32767`,
   `604800`) in the adapter.
2. **No allocation per tick on hot paths.** `Telemetry` is reused, and handles (`Var<T>`) are
   resolved once per connection.
3. **Shared memory is written by another process.** Only copy out of it; never hold references
   into it. Validate tick counts after copying.
4. **`unsafe` needs a `// SAFETY:` comment** (enforced by clippy).
5. **Tests use synthetic images** (`reo_irsdk::testing::ImageBuilder`). A regression test must
   fail without its fix: check that before trusting it.
6. iRacing strings are Windows-1252, and its session YAML is not always valid YAML (unquoted
   names). Go through `reo_iracing::session::sanitize`.
7. **Telemetry recordings stay out of git** (`*.irdt`, `*.lmudt` are ignored). They carry driver
   names and customer IDs; tests build synthetic data instead.

## Development flow

<!-- shared:development-flow -->

The flow skills are this repository's binding process for changing code. Read
`.agents/skills/open-pr/settings/dev-flow.md` (the routing, exemptions and issue linking) and
`docs/agents/dev-flow.md` (what this repository adds) before the first edit of a change, and whenever
no ready issue exists. `/ask-matt` maps the flow itself.

- **The gate.** Every code edit starts from a ready issue: an open `ready-for-agent` issue whose body
  is a `/to-spec` spec or a `/to-tickets` ticket or that carries a `/triage` brief, or a `bug` issue
  worked through `/diagnosing-bugs`, which needs the number and no spec. With none, stop before the
  first edit and route the user. Reading code, running tests and answering questions need no issue.
- **Private work.** A ready issue in the private tracker counts. Its number stays out of the branch,
  the PR, commits and public issues, and it is closed by hand after the merge.
- **The human starts every phase.** `/wayfinder`, `/grill-with-docs`, `/to-spec`, `/to-tickets`,
  `/implement` and `/triage` stay user-invoked. At a phase boundary, name the next command, say in
  one line why, and wait. Inside a phase, use `/tdd`, `/code-review`, `/diagnosing-bugs`,
  `/domain-modeling`, `/codebase-design`, `/prototype` and `/research` whenever they fit.
- **Exempt from the gate:** dependency bumps, changes touching only docs and agent config, release
  prep, and an explicit user waiver recorded in the PR body as `Flow waived by the user: <reason>`.
- **Linking.** Branches are `<type>/<issue#>-<slug>`; the PR body opens with `Closes #N` or
  `Part of #N`.
- **The flow ends at `/open-pr`.** `/implement` stops at a commit; `/open-pr` refreshes the docs,
  writes the changelog and the PR body, and opens the PR (Claude Code's built-in `/ship` is a
  different command, outside this flow). When the user says to merge it, `/land` merges it
  and confirms its release. `/dependabot` reviews the open Dependabot PRs and hands the ones the
  user names to `/land`; nothing merges a Dependabot PR on its own. `/ask-matt` also routes to
  `/pr`, `/retro` and `/implement-spec`, which are not installed here.
- **Tests are proven, reds are read.** Before `/implement` commits a change that adds or changes
  tests, and in `/code-review`, run `/test-prover`: in a scratch worktree it reverts the fix and
  breaks its guards, and every new test must go red. `/open-pr` runs it too. `/land` and
  `/dependabot` read a red check with `/ci-triage`, which never re-runs anything.
  `/records-audit` checks the board, the trackers, the follow-up wiki and the changelog against
  each other; run it weekly, and `/what-next` says when it is due.
- **`/handoff` is the shared skill.** It parks the session for another machine or harness, closes it
  out with `/end-session`, and `/lets-go` resumes it. It does not write the portable file `/ask-matt`
  describes, so a `/prototype` detour stays in the session or uses a branch.
- **A wizard is not the record.** A step only a human can take follows the `## Procedure` in
  `.agents/skills/open-pr/settings/human-follow-up.md`, whose instructions stand without the script.
- **Research goes where it is safe to keep.** `/research` findings that would expose private material
  go to the private companion, where there is one, never to an ignored folder with no repository
  behind it. Details of an unpublished vulnerability are a draft advisory, never a research file.

<!-- /shared:development-flow -->

## Agent skills

Skills are authored in `.agents/skills/` (Codex reads it directly); `.claude/skills/` is a
generated mirror for Claude Code. Never edit the mirror: run `node scripts/sync-agents.mjs`.

### Shared skills

<!-- shared:shared-skills -->

`end-session`, `handoff`, `lets-go`, `open-pr`, `land`, `dependabot`, `docs-updater`, `what-next`,
`test-prover`, `ci-triage`, `records-audit` and `tldr` install from `jwh3times/skills`, and are
never edited here: reinstalling overwrites a local edit.
A fix goes upstream, and what they need to know about this repository goes in the settings files
under `docs/agents/`, which each skill proposes on first run.

The other shared skills come from one `mattpocock/skills` snapshot (v1.3.2, pinned by commit), the
same in every repository, and are never edited here either: what they need to know goes in this
file or `docs/agents/`. Moving to a newer snapshot is a change in `jwh3times/skills`. Keep
`.agents/skills/` out of every formatter: `scripts/check-skills.mjs` fails when an installed copy
differs from `skills-lock.json`, or the lock from these commands.

Re-running the install is the update; never run `npx skills update`, which leaves `.claude/skills/`
linked to its source instead of copied:

```bash
npx skills add jwh3times/skills -s end-session -s handoff -s lets-go -s open-pr -s land -s dependabot -s docs-updater -s what-next -s test-prover -s ci-triage -s records-audit -s tldr -a codex --copy -y
npx skills add mattpocock/skills#d1478fcb140db0b93c0c8df17bcfd311844749d0 \
  -s ask-matt -s code-review -s codebase-design -s diagnosing-bugs -s domain-modeling -s grill-me \
  -s grill-with-docs -s grilling -s implement -s improve-codebase-architecture -s prototype \
  -s research -s setup-matt-pocock-skills -s tdd -s teach -s to-questionnaire -s to-spec \
  -s to-tickets -s triage -s wait-what -s wayfinder -s wizard -s writing-for-agents \
  -a codex --copy -y
node scripts/sync-agents.mjs
```

`scripts/sync-agents.mjs`, `lint-agent-neutrality.mjs`, `check-docs.mjs`, `check-skills.mjs`,
`guard.mjs`, and `sync-main.mjs`, `next-version.mjs`, `check-changelog.mjs` and
`stamp-changelog.mjs` where this repository carries them, are canonical in `jwh3times/skills` with
their tests: fix them there, then copy them here.
`sync-agents.mjs` also writes the marked blocks in this file: change them in `jwh3times/skills`, not
here. It registers `guard.mjs` in `.claude/settings.json`, which stops the commands and edits these
rules forbid; this repository's own additions go in `docs/agents/guard.md`.

<!-- /shared:shared-skills -->
