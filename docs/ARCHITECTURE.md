# Architecture

## Goals, in priority order

1. **Performance on a triple-1440p rig.** The overlay must not cost the sim frames
   or add visible latency. Target: under 10% of one CPU core and under 80 MB while
   racing, with no garbage-collection stutter in the overlays.
2. **Our own look.** A distinctive, themeable visual identity, not a stock toolkit.
3. **Multi-sim from the start.** iRacing first, then Le Mans Ultimate, ACC and AMS2,
   all behind one model.

## Decision: hybrid stack (2026-11-25)

```
 sim adapters (Rust)           core (Rust)                 presentation
 ┌──────────────┐        ┌────────────────────┐     ┌──────────────────────────────┐
 │ iracing      │──┐     │ poll loop (60 Hz)  │     │ overlays: native Direct2D +  │
 │ lmu (later)  │──┼────▶│ derived state      │────▶│ DirectComposition, one       │
 │ acc (later)  │──┤     │ (gaps, fuel,       │     │ click-through window/widget  │
 │ ams2 (later) │──┘     │  relative, deltas) │     ├──────────────────────────────┤
 └──────────────┘        └────────────────────┘     │ settings app: Tauri + web UI │
        ▲ reo-model: the only contract between      │ (forms, layout editor)       │
          adapters and everything downstream        └──────────────────────────────┘
```

- **Overlays draw natively.** Each widget gets its own small click-through
  window, which costs almost nothing natively; this is the main reason to avoid a
  browser engine for the overlays. Widgets redraw only when their inputs change.
  Only fast widgets (inputs, delta bar, radar) run at the display rate.
- **Settings is a web app** (Tauri). It's forms-heavy and not on the hot path,
  so CSS and hot reload win there. It's closed while racing.
- **Themes and layouts are data files,** hot-reloaded, so the look can be
  iterated without rebuilding.
- **Alternatives considered:** Electron (what irDashies uses), Tauri for
  everything, .NET WPF/Avalonia, Qt Quick, and immediate-mode GUIs. They were
  rejected mainly on resource use (the web stacks) or on how hard it is to get a
  polished custom look (immediate mode).

## Crates

| Crate | Role |
|---|---|
| `crates/model` (`reo-model`) | Sim-agnostic types: `Telemetry` (per tick, reused), `SessionInfo` (rare, `Arc`), the `TelemetrySource` trait. SI units, `None` for "not provided". |
| `crates/irsdk` (`reo-irsdk`) | iRacing shared-memory reader: header and variable table parsing, torn-read-safe row copies, typed `Var<T>` handles, the Windows mapping and event. Includes `testing::ImageBuilder` for synthetic images. |
| `crates/iracing` (`reo-iracing`) | Adapter: maps SDK variables and session YAML onto `reo-model`; `IRacingSource` handles connecting and reconnecting, and decodes and parses each session-YAML revision on a worker thread it owns, so the poll only copies the text out of shared memory. |
| `apps/reo-cli` (`reo`) | Developer CLI: `watch`, `vars`, `session` (live or from a tape) and `check` (runs a whole tape and reports timings and sanity statistics). |

## Measured so far

The 10-minute AI race tape from irDashies' test data (36,000 ticks, 41 cars,
333 variables), measured with `reo check` on a release build (2026-11-27). Tape
replay keeps the session-info region length the sim reports (512 KiB, with the
66 KB YAML NUL-terminated inside it), so the copy matches the live layout:

- Read and map one tick: **mean 1.2–1.4 µs, p99 2.4 µs**.
- iRacing revises the session YAML about 70 times in 10 minutes. The adapter
  decodes and parses each revision on a worker thread it owns, and all 70 are
  applied:
  - The tick that hands a revision to the worker costs **mean about 17 µs, p50
    11 µs, p99 19 µs**. That is the tick itself, the copy of the text (not the
    whole region) out of shared memory into a reused buffer (**about 6 µs**), and
    the hand-off (**about 4.5 µs**). The median adds up; the mean carries the
    first revision, which starts the worker and allocates the first buffer
    (about 300 µs). In steady state the poll thread allocates nothing for a
    revision; it allocates only for one that arrives while the worker holds the
    only buffer and another is waiting, or one that outgrows the buffer. Before
    the buffer was reused, allocating and faulting in fresh pages made this tick
    about 200 µs.
  - The decode and parse on the worker cost **mean 4.6 ms, max about 7.5 ms**.

## Roadmap

1. **Core (done):** iRacing reader, model, adapter, CLI, tape playback.
2. **Telemetry service:** a dedicated thread that owns the source and publishes
   snapshots to consumers (triple buffer or seqlock, no locks on the render
   path). Add derived state: relative gaps, a class-aware standings sort, and
   fuel per lap.
3. **Our own recorder and tape format,** so testing stops depending on irDashies'
   `.irdt` format.
4. **Native overlay spike:** one DirectComposition click-through window drawing
   a delta bar and an input trace with Direct2D/DirectWrite. Measure the game's
   frame time with PresentMon on the rig.
5. **Widget toolkit:** layout, text, theming from data files, hot reload.
6. **First widgets:** relative, standings, inputs, delta, fuel.
7. **Settings app** (Tauri): widget placement, themes, per-sim profiles.
8. **Second sim:** Le Mans Ultimate, to prove the model holds up.
