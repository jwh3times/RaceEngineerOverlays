# Guard

The commands and paths `scripts/guard.mjs` stops here, on top of the rules it applies everywhere.
Each `##` heading is one setting, defined in `.agents/skills/open-pr/settings/guard.md`.

## Deny commands

- `^npx (-y |--yes )?skills update\b` — it leaves `.claude/skills/` linked to its source instead of
  copied. Re-run the install commands in the `shared-skills` block of `AGENTS.md` instead.

## Ask commands

- `^git add\b.*\.(irdt|lmudt)\b` — a telemetry recording carries driver names and customer IDs,
  and `.gitignore` keeps them out on purpose (`AGENTS.md` rule 7).
- `^(npm|npx)( --?[\w-]+)* run irsdk:build\b` — irDashies' native build deletes `irdashies/build/`
  before it compiles, and it needs the ClangCL toolset; a failed build leaves the reference clone
  unable to start.

## Protected paths

- `^irdashies/` — the upstream reference clone, kept unmodified (`AGENTS.md`, Layout).
