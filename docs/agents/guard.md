# Guard

The commands and paths `scripts/guard.mjs` stops here, on top of the rules it applies everywhere.
Each `##` heading is one setting, defined in `.agents/skills/open-pr/settings/guard.md`.

## Deny commands

- `^npx (-y |--yes )?skills update\b` — it leaves `.claude/skills/` linked to its source instead of
  copied. Re-run the install commands in the `shared-skills` block of `AGENTS.md` instead.

## Ask commands

- `^git add\b.*\.(irdt|lmudt)\b` — a telemetry recording carries driver names and customer IDs,
  and `.gitignore` keeps them out on purpose (`AGENTS.md` rule 7).

## Protected paths

- `^tapes/` — the local telemetry recordings; git ignores them, so nothing restores them
  (`AGENTS.md`, Layout).
