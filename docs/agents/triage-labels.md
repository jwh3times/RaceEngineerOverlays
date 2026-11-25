# Triage labels

The labels and what they mean are in `.agents/skills/open-pr/settings/triage-labels.md`. A skill's
role name is the GitHub label to apply.

## How this repository uses them

`needs-info` covers any missing evidence, not only an unanswered question to a reporter: a
behaviour that can only be designed once someone has observed it in a running sim waits with
`needs-info` for that observation.

`ready-for-human` means an agent cannot complete the work: it needs credentials, a running sim on
the maintainer's rig, or a decision only the maintainer can make. It is not a judgement about
difficulty.

## Extra labels

- `blocked` — the issue is waiting on another issue or an external event and cannot be progressed
  now, so nobody should pick it up. It sits beside the five rather than replacing one: the triage
  label still says what the issue will need once unblocked. Apply it with the board's `Gate` set to
  `Depends on another issue`, `External wait` or `Upstream`, and remove it in the same change that
  clears the blocker.
