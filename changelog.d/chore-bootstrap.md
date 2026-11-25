### Added

- The iRacing telemetry core from the initial commit: the sim-agnostic model, the iRacing
  shared-memory reader and adapter, and the `reo` developer CLI (`watch`, `vars`, `session`,
  `check`), which reads live from iRacing or replays a recorded `.irdt` tape.
- Every merge to `main` is released: a `v<major>.<minor>.<build>` tag, a GitHub Release, and a
  Windows x64 zip of `reo` with its SHA-256 checksum.
