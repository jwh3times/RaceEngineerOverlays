# Race Engineer Overlays

Fast, native sim-racing overlays: a Rust telemetry core, Direct2D widgets drawn in their own
click-through windows, and a web-based settings app. iRacing comes first; Le Mans Ultimate, ACC and
AMS2 follow behind the same model.

The goal is overlays that cost the sim nothing you can measure on a triple-1440p rig: under 10% of
one CPU core and under 80 MB while racing, with no garbage-collection stutter. See
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the stack decision, the measurements so far and
the roadmap.

## Status

Early. The telemetry core is done: it reads iRacing's shared memory (about 1.2 µs per tick) and
maps it onto a sim-agnostic model. There are no widgets yet; the next steps are the telemetry
service thread and the native overlay spike.

## The `reo` CLI

A developer tool for inspecting telemetry, live from iRacing or from a recorded `.irdt` tape:

```bash
reo watch                     # a running summary line, live from iRacing
reo watch --irdt <tape> --speed 10
reo vars --irdt <tape>        # every SDK variable with its current value
reo session --raw --irdt <tape>
reo check <tape>              # replay a whole tape and report timings and sanity statistics
```

Every merge to `main` publishes a GitHub Release with a Windows x64 zip containing `reo.exe`,
`README.md`, `LICENSE`, `SECURITY.md` and the release notes as `RELEASE-NOTES.md`, plus
`SHA256SUMS.txt`.

## Building from source

Windows, with the Rust toolchain from [rustup](https://rustup.rs/). `rust-toolchain.toml` pins Rust
1.100, which rustup installs on the first build.

```bash
cargo build --release --locked -p reo-cli    # target/release/reo.exe
cargo test --workspace --locked
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) and [AGENTS.md](AGENTS.md). Report security issues as in
[SECURITY.md](SECURITY.md).

## License

[AGPL-3.0-only](LICENSE).
