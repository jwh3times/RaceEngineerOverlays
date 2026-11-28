# PROTOTYPE: stack spike (wayfinder ticket #28)

Throwaway. This lives only on the `prototype/28-stack-spike` branch and is never merged. It answers
one question: on performance grounds, should reo's widgets draw natively (Direct2D/DirectWrite into
DirectComposition) or in WebView2, and in one window per widget or one composition surface per
monitor?

`spike-stack` draws the Relative as `--widgets` copies spread across the monitors. A feed thread
polls the sim, or a tape, computes the Relative once per tick and hands the latest snapshot to the
render thread. The render thread wakes once per snapshot, with no polling timer. Each widget redraws
only when its text changes: native rebuilds only the changed DirectWrite layouts, and WebView2 skips
`evaluate_script` when nothing changed and touches only changed DOM cells. The whole process runs at
below-normal priority unless `--normal-priority`. Driver names show as "Driver <number>" unless
`--names`, so screenshots carry no names.

```bash
cargo build --release -p spike-stack
target/release/spike-stack --stack native  --layout per-widget  --irdt tapes/ai-race-10min.irdt --skip 240 --seconds 30
target/release/spike-stack --stack webview --layout per-monitor --seconds 60      # live iRacing
```

It reports the CPU and memory of its whole process tree (WebView2's child processes included), the
render and feed threads' CPU, the render thread's time per frame, and tick-to-drawn freshness.
`measure-live.ps1` runs every configuration against a live iRacing session under PresentMon, and
`summarize.mjs` tabulates the sim's frame times beside the spike's numbers.

## Tape results (2026-11-28, no sim running)

These runs used the maintainer's machine (RTX 3080 Ti, two 2560×1440 monitors connected at the
time) and the 10-minute AI race tape at 1x from 240 s in. Each ran 30 s after a 5 s warm-up, with 6
widgets. The Relative's text changed about 7 times a second (gaps at 0.1 s resolution).

| Configuration | CPU, % of one core | Private MB | Working set MB | Processes |
|---|---|---|---|---|
| Feed only, nothing drawn | 0.3 | 60 | 15 | 1 |
| Native, window per widget | 1.7–2.3 | 118 | 54 | 1 |
| Native, surface per monitor | 1.8–3.0 | 119 | 54 | 1 |
| WebView2, window per widget | 7.6–9.3 | 394 | 696 | 13 |
| WebView2, surface per monitor | 2.9–3.5 | 336 | 400 | 8 |

- **About 60 MB of private memory is the tape's in-process image.** A live run maps iRacing's shared
  memory instead, so live private memory is about 58 MB for native and 276–334 MB for WebView2.
  WebView2 is three to four times the 80 MB budget before reo has any other widget.
- **Native per draw:** a widget redraw costs about 110 µs in `EndDraw` and 130–145 µs in `Present`,
  the same in both layouts. Six small swap chains in one window per monitor cost the same as six
  windows.
- **WebView2 doesn't keep its priority.** The spike runs below normal and its children inherit
  that, but WebView2 raises its own: during the run its GPU process was above normal and two others
  were at normal. reo can't keep a WebView2 tree below normal, which breaks the telemetry-service
  decision that Windows always favours the sim.
- **WebView2 per monitor is far cheaper than per widget.** It has fewer renderer and compositor
  processes, but it is still a 336 MB, 8-process tree.
- **Not yet measured:** the sim's frame time. That needs the rig: run `measure-live.ps1`.

## A bug found on the way

The first per-monitor runs looked two to three times dearer than per-widget. They actually built a
window for every widget, each holding three widgets (18 in all), because the monitor list was
deduplicated without sorting. Fixed; the table above is after the fix.
