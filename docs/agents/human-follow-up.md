# Human follow-up

Required human actions left by completed agent work follow the procedure in the installed skills'
`human-follow-up` settings definition (for example
`.agents/skills/end-session/settings/human-follow-up.md`). The agent that completes the work owns
the follow-up; docs-updater, open-pr and end-session verify it. Follow-ups are issues in
`jwh3times/RaceEngineerOverlays` with the `ready-for-human` label, on the board.

## Follow-up board

same as Board — `Status` `Todo`, with `Gate` `Owner action` (or `Owner decision` for a decision).
An action that waits on a prerequisite gets the `blocked` label and the matching `Gate`
(`Depends on another issue`, `External wait`). Checking a change against a live iRacing session,
or measuring an overlay's cost in a running sim with PresentMon, is always a human action here.
