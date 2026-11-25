# Contributing

## Code

1. Read [AGENTS.md](AGENTS.md). It lists the rules every change follows. The short version:
   - the sim-agnostic model in `crates/model` is the only contract between sim adapters and
     everything downstream;
   - model units are SI, and anything a sim doesn't provide is `None`, never a sentinel;
   - no allocation per tick on hot paths;
   - shared memory is only copied out of, never referenced;
   - telemetry recordings never enter the repository.
2. `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings` and
   `cargo test --workspace` must pass. CI's `rust` job runs exactly these; its `agents` and
   `changelog` jobs check the repository scripts and docs (the `node` commands in AGENTS.md).
   Describe a user-visible change in a `changelog.d/` fragment, never in `CHANGELOG.md` itself
   ([changelog.d/README.md](changelog.d/README.md)).
3. A bug fix needs a test that fails without it. Tests build synthetic shared-memory images
   (`reo_irsdk::testing::ImageBuilder`) rather than reading a running sim or a recording.

## License of contributions

This project is licensed under [AGPL-3.0-only](LICENSE). By opening a pull request, you agree
that your contribution is licensed under the same terms (inbound = outbound). Sign off your
commits (`git commit -s`) to certify the [Developer Certificate of Origin](https://developercertificate.org/).
