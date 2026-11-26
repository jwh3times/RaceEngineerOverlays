### Fixed

- The README lists everything the release zip holds: `reo.exe`, `README.md`, `LICENSE`,
  `SECURITY.md` and the release notes as `RELEASE-NOTES.md`.
- `CONTRIBUTING.md` and `AGENTS.md` name the commands CI actually runs (`cargo fmt --all --check`,
  `--locked` on clippy and test), and separate the release-preview scripts no CI job runs (#8)
