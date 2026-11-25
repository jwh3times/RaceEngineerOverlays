# Test proof

How `test-prover` runs one test on its own here, and how it tells a run that tested nothing from a
pass. The suites and their CI jobs are in `docs/agents/open-pr.md`.

## Test commands

- repo root: Rust unit tests, in the `#[cfg(test)]` modules of `crates/<dir>/src/**/*.rs` and
  `apps/<dir>/src/**/*.rs`. The package is the `name` in the `Cargo.toml` of the file's crate
  folder (`crates/irsdk` → `reo-irsdk`, `crates/iracing` → `reo-iracing`, `crates/model` →
  `reo-model`, `apps/reo-cli` → `reo-cli`), and a test's name is its path inside the crate
  (`session::tests::<fn>`). One file:
  `cargo test --locked -p <package> --lib -- "$(basename "<file>" .rs)::"` (`--bin reo` instead of
  `--lib` for `apps/reo-cli`; for `lib.rs` or `main.rs`, drop the filter after `--`, which runs the
  crate's whole unit suite); one test: `cargo test --locked -p <package> --lib -- --exact "<name>"`.
  Count: cargo's `test result:` lines. Worktree: none; the first run builds the workspace into the
  worktree's own `target/`, about a minute.
- repo root: Rust integration tests, `crates/<dir>/tests/*.rs`. One file:
  `cargo test --locked -p <package> --test "$(basename "<file>" .rs)"`; one test:
  `cargo test --locked -p <package> --test "$(basename "<file>" .rs)" -- --exact "<name>"`, the
  name being the function's. Worktree: as above.
- repo root: `scripts/*.test.mjs`, node's runner. One file: `node --test <file>`; one test:
  `node --test --test-name-pattern="<name>" <file>`. Worktree: none (the scripts have no
  dependencies).
- Not proven: checks against a live sim or a recorded tape (`reo watch`, `reo check`); they are
  manual and need data that never enters the repository.

## Guide

AGENTS.md
