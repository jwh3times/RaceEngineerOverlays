# Docs

## Doc map

- `README.md`: what Race Engineer Overlays is, its status, building from source and the `reo`
  CLI's main commands; for users and contributors. Update when a `reo` command or flag changes
  (`apps/reo-cli/src/main.rs`), when the release zip's contents change
  (`.github/workflows/release.yml`'s `package` job), or when a build step or the toolchain pin
  (`rust-toolchain.toml`) changes.
- `CONTRIBUTING.md`: how to contribute, the checks a PR passes, the licence of contributions and
  the DCO sign-off. Update when the checks CI runs change (`.github/workflows/ci.yml`) or the
  contribution process does.
- `SECURITY.md`: how to report a vulnerability, and what the overlays trust. `owner-only`.
- `CHANGELOG.md`: one dated section per version a merge mints, stamped by the release workflow's
  `scripts/stamp-changelog.mjs` after the merge (`docs/agents/releases.md`). `frozen`: no branch
  writes its sections.
- `changelog.d/README.md`: the fragment format and how fragments are stamped, for whoever writes
  one. Update when the fragment rules in `scripts/check-changelog.mjs` or the stamping in
  `scripts/stamp-changelog.mjs` change.
- `changelog.d/*.md`: each branch's changelog fragment (every file there but the README), stamped
  into `CHANGELOG.md` after its merge and checked by the `changelog` CI job. `ship-owned`, like the
  `VERSION` file.
- `docs/ARCHITECTURE.md`: goals, the stack decision, the crates, the measured numbers and the
  roadmap. Update when a crate is added or its role changes (`Cargo.toml` members), a measurement
  is retaken, or a roadmap step lands.
- `docs/adr/*.md`: architecture decision records, one decision each with the reason it was made.
  Add one when a decision is hard to reverse and would surprise a reader; supersede rather than
  rewrite.
- `AGENTS.md`: guidance for every coding agent: what this is, commands, layout, the rules for every
  change, the development flow and the agent skills. Update when a command, a crate or a rule
  changes, the toolchain pins do (`rust-toolchain.toml`, `.nvmrc`), or `.claude/settings.json`
  does. The `shared:` blocks are written by `scripts/sync-agents.mjs`; never edit them here.
- `CLAUDE.md`: imports `AGENTS.md` for Claude Code. `owner-only`.
- `docs/agents/*.md`: settings the shared skills read. Update when this repository's process
  changes, or the `main` or tag rulesets do (`open-pr.md` lists the `main` rules, `releases.md`
  and `merging.md` the tag rules).
- `docs/agents/merging.md`: from open PR to released and packaged version. Update when a merge or
  release lesson is learned that no check enforces, a command it quotes changes, or the `main` or
  tag rulesets do.
- `docs/agents/dependabot.md`: what reviewing a Dependabot pull request takes here, read by the
  `dependabot` skill. Update when `.github/dependabot.yml`, a workflow's toolchain or install steps,
  `rust-toolchain.toml`, `.nvmrc` or the workspace's main dependencies change.
- `docs/agents/guard.md`: the commands `scripts/guard.mjs` denies or asks about here, and the
  paths it protects, on top of its universal rules. Update when a never-remove path in
  `docs/agents/workspace.md` changes, or a new kind of private recording appears.
- `docs/agents/test-proof.md`: how the `test-prover` skill runs one test on its own here. Update
  when a test folder, a crate or a test command changes.
- `.agents/skills/**`: installed skills, listed in `skills-lock.json`. `frozen`: a reinstall
  overwrites them, and `scripts/check-skills.mjs` fails on an edited copy.

## Drift probes

- CI jobs and their commands: `.github/workflows/ci.yml`.
- Required checks and the other `main` rules:
  `gh api repos/jwh3times/RaceEngineerOverlays/rules/branches/main`.
- The tag ruleset's rules and bypass actors:
  `gh api repos/jwh3times/RaceEngineerOverlays/rulesets/<id>`, the id from
  `gh api repos/jwh3times/RaceEngineerOverlays/rulesets`.
- Crates in `AGENTS.md`'s layout and `docs/ARCHITECTURE.md`: the `members` of `Cargo.toml` and
  the folders under `crates/` and `apps/`.
- CLI commands in `README.md` and `AGENTS.md`: the `Command` enum in `apps/reo-cli/src/main.rs`.
- Release zip contents in `README.md`: the `package` job in `.github/workflows/release.yml`.
- The toolchain in `README.md`, `AGENTS.md` and `docs/agents/dependabot.md`: `rust-toolchain.toml`;
  and the Node version in `AGENTS.md` and `docs/agents/dependabot.md`: `.nvmrc`.
- Auto-merge, which `docs/agents/merging.md` says is disabled:
  `gh api repos/jwh3times/RaceEngineerOverlays --jq .allow_auto_merge`.
- CodeQL default setup, which `docs/agents/merging.md` says is configured (`state`):
  `gh api repos/jwh3times/RaceEngineerOverlays/code-scanning/default-setup`.
