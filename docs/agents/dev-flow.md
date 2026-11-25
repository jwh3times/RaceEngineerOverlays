# Development flow

The flow itself is `.agents/skills/open-pr/settings/dev-flow.md`, and `AGENTS.md` summarizes it.
This file holds what this repository adds.

## Recordings stay local

A change that needs evidence from a real sim session (a live `reo watch`, a recorded tape) runs it
locally and records only the numbers in the PR. Telemetry recordings carry driver names and
customer IDs; they are never committed or attached to an issue (`AGENTS.md` rule 7).

## After the pull request opens

`/open-pr` stops once the PR is open. Merging and its follow-ups are in `docs/agents/merging.md`.
