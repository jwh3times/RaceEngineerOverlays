### Changed

- The iRacing session YAML is parsed on a background thread, so a session change no longer stalls
  the telemetry poll. On a recorded 10-minute race, the tick that carries a session revision drops
  from about 7.4 ms to about 0.25 ms; the parse itself (about 4.6 ms) runs off the poll thread.
  Until a new revision is parsed, the previous session information stays current, and the change
  is reported a few ticks after the sim published it (#5)
- `reo check` reports the tick that hands off a session revision (`tick + revision`) separately
  from the parse on the worker (`parse (worker)`), and counts `N applied of M revisions`.
