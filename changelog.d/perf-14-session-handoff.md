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
