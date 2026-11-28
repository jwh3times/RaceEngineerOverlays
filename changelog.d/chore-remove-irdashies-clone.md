### Changed

- The local validation tapes now live in a gitignored `tapes/` folder at the repository root, so
  the validation commands are `reo check tapes/ai-race-10min.irdt` and
  `reo watch --irdt tapes/ai-race-10min.irdt`. The reference clone of irDashies is no longer kept
  in the working tree.
