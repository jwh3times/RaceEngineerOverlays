# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html) via the
`v<major>.<minor>.<build>` tags minted by `.github/workflows/release.yml`. Branches never edit this
file: each writes its entries to `changelog.d/`, and the workflow stamps them into the section for
the version its merge mints ([ADR 0001](docs/adr/0001-stamp-changelog-on-tag.md)).

_`v0.0.0` marks the initial commit, which predates this scheme and has no section; its code is
released in 0.1.0._

## [Unreleased]

No unreleased changes.

## [0.1.9] - 2026-11-27

### Changed

- The local validation tapes now live in a gitignored `tapes/` folder at the repository root, so
  the validation commands are `reo check tapes/ai-race-10min.irdt` and
  `reo watch --irdt tapes/ai-race-10min.irdt`. The reference clone of irDashies is no longer kept
  in the working tree.

## [0.1.8] - 2026-11-27

### Changed

- Take the latest shared skills and scripts from `jwh3times/skills` (#17)

## [0.1.7] - 2026-11-27

### Changed

- A session revision costs the telemetry poll much less. The poll thread copies only the session
  YAML's text out of shared memory, not the 512 KiB region iRacing reserves for it, into a buffer it
  reuses; the Windows-1252 decode now runs on the background thread with the parse. On a recorded
  10-minute race, the tick that carries a revision drops from about 200 µs to about 17 µs, and in
  steady state the poll thread no longer allocates for a revision (#14)
- `IRacingSource::session_yaml()` updates when the background thread has decoded a revision,
  rather than on the tick that read it; once `wait_for_session` returns it is the newest revision,
  including one that failed to parse.
- `reo check` replays a tape's session-info region at the length iRacing reports, so it measures
  the same copy as the live sim, and splits the revision tick into `copy out` and `hand-off`. The
  worker's line is now `decode + parse`.

## [0.1.6] - 2026-11-26

### Changed

- Take the latest shared skills and scripts from `jwh3times/skills` (#12)

## [0.1.5] - 2026-11-26

### Changed

- The iRacing session YAML is parsed on a background thread, so a session change no longer stalls
  the telemetry poll. On a recorded 10-minute race, the tick that carries a session revision drops
  from about 7.4 ms to about 0.25 ms; the parse itself (about 4.6 ms) runs off the poll thread.
  Until a new revision is parsed, the previous session information stays current, and the change
  is reported a few ticks after the sim published it (#5)
- `reo check` reports the tick that hands off a session revision (`tick + revision`) separately
  from the parse on the worker (`parse (worker)`), and counts `N applied of M revisions`.

## [0.1.4] - 2026-11-26

### Changed

- Take the latest shared skills and scripts from `jwh3times/skills` (#10)

## [0.1.3] - 2026-11-26

### Fixed

- Car classes iRacing leaves unnamed now get a name where one can be inferred. iRacing leaves
  `CarClassShortName` empty in AI and hosted sessions, multi-class ones included, and every class
  came out with an empty name. A class now takes a name any of its drivers carries, or its car's
  name when it holds a single model, such as `Dallara P217 LMP2`. A class of several models, such
  as GT3, has no name, and its class ID still groups it (#6)
- `reo check` lists the session's classes by ID with their names, such as
  `2523 Dallara P217 LMP2 | 4011 (unnamed)`, instead of a single empty name.

## [0.1.2] - 2026-11-26

### Fixed

- The README lists everything the release zip holds: `reo.exe`, `README.md`, `LICENSE`,
  `SECURITY.md` and the release notes as `RELEASE-NOTES.md`.
- `CONTRIBUTING.md` and `AGENTS.md` name the commands CI actually runs (`cargo fmt --all --check`,
  `--locked` on clippy and test), and separate the release-preview scripts no CI job runs (#8)

## [0.1.1] - 2026-11-25

### Changed

- Take the latest shared skills and scripts from `jwh3times/skills` (#4)

## [0.1.0] - 2026-11-25

### Added

- The iRacing telemetry core from the initial commit: the sim-agnostic model, the iRacing
  shared-memory reader and adapter, and the `reo` developer CLI (`watch`, `vars`, `session`,
  `check`), which reads live from iRacing or replays a recorded `.irdt` tape.
- Every merge to `main` is released: a `v<major>.<minor>.<build>` tag, a GitHub Release, and a
  Windows x64 zip of `reo` with its SHA-256 checksum.
