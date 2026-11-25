# Race Engineer Overlays: agent guide

Sim-racing overlays built from scratch for performance: a Rust core, native
Direct2D overlays and a web-based settings app. iRacing comes first, with more
sims to follow. "reo" is the working name used in crate and binary names.

Read [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) before structural changes.
It records the stack decision, the crate roles, the measured numbers and the
roadmap.

## Layout

- `crates/model`: the sim-agnostic model. The only contract between sim
  adapters and everything downstream. Adapters must not leak sim-specific
  types past it.
- `crates/irsdk`: iRacing shared-memory reader.
- `crates/iracing`: iRacing adapter onto the model.
- `apps/reo-cli`: developer CLI (`reo`).
- `docs/`: architecture and decisions.
- `irdashies/`: an unmodified clone of upstream irDashies (MIT), kept for
  **reference only**. It is not part of this project's build and is ignored
  by git. Don't copy its code; its test-data tapes are used for local
  validation.

## Commands

```bash
cargo test --workspace
cargo clippy --workspace --all-targets      # keep it warning-free
cargo build --release -p reo-cli

# Validate the core against a recorded race (no sim needed)
target/release/reo check irdashies/test-data/telemetry/ai-race-10min.irdt
target/release/reo watch --irdt irdashies/test-data/telemetry/ai-race-10min.irdt --speed 10
target/release/reo vars  --irdt <tape>      # every variable with its current value
target/release/reo session --raw --irdt <tape>

# Live (iRacing running)
target/release/reo watch
```

## Conventions

- **Model units are SI** (m, s, L, rad, m/s). Pedals and fractions are
  `0.0..=1.0`. Anything a sim doesn't provide is `None`, never a sentinel.
  Translate sim sentinels (`-1`, `0`, `32767`, `604800`) in the adapter.
- **No allocation per tick on hot paths.** `Telemetry` is reused, and handles
  (`Var<T>`) are resolved once per connection.
- **Shared memory is written by another process.** Only copy out of it; never
  hold references into it. Validate tick counts after copying.
- **`unsafe` needs a `// SAFETY:` comment** (enforced by clippy).
- **Tests use synthetic images** (`reo_irsdk::testing::ImageBuilder`). A
  regression test must fail without its fix: check that before trusting it.
- iRacing strings are Windows-1252, and its session YAML is not always valid
  YAML (unquoted names). Go through `reo_iracing::session::sanitize`.

## Known follow-ups

- Session-YAML parsing takes about 5–8 ms and runs inline on the poll. Move it
  to a worker before overlays depend on the poll thread.
- `CarClassShortName` is empty in single-class sessions. Fall back to the car
  name when displaying the class.
