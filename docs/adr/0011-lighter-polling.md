# 0011. Poll the heaviest data every 2 seconds, not every 1

- **Status:** Accepted
- **Date:** 2026-09-24

## Context

The user asked whether the agent's resource use could be lower. Measured on
a real build: the agent process itself averaged around 5% CPU polling at
1-second intervals (`POLL_INTERVAL_MS` in `main.js`), on top of the WebKit
helper processes every Tauri app has.

The first fix was refreshing the process list — the single most expensive
thing `SystemCollector` does, since it enumerates and sorts every process on
the system — on its own slower interval rather than on every 1-second tick
(see `SystemCollector::top_processes` and `PROCESS_REFRESH_INTERVAL`). That
change is correct on its own terms (it removes real, unnecessary repeated
work), but measured on the whole running app it made barely a dent: process
enumeration was only one of several things happening every tick (CPU,
memory, disk, and network refreshes; re-rendering six cards' worth of DOM),
and background-process CPU measurements are noisy enough at this scale that
a few points of difference isn't distinguishable from measurement noise
anyway.

## Decision

Poll every 2 seconds instead of every 1 (`POLL_INTERVAL_MS` in `main.js`).
This is the lever that actually scales: it's not "find and remove the one
expensive thing", it's "do the same set of moderately-cheap things half as
often" — cutting the whole tick's cost roughly in half, provably, rather
than hoping one optimization dominates.

- The CPU/Memory sparklines keep roughly a minute of history:
  `SPARKLINE_SAMPLE_COUNT` dropped from 60 to 30 to match (still 1
  sample/2s × 30 = 1 minute), rather than quietly becoming a 2-minute window.
- `PROCESS_REFRESH_INTERVAL` (from the first fix, above) moved from 3s to 6s,
  keeping process refreshes at roughly a third of the base poll rate rather
  than nearly every tick now that the base rate itself is slower.
- Claude (5s) and weather (60s, with its own 15-minute server-side cache)
  were already on their own slower timers and didn't need to change.

## Consequences

- Every "live" number on screen can be up to 1 second staler than before.
  For CPU/memory/disk/network/temperature, none of which spike and vanish
  within a couple of seconds, this isn't perceptible in normal use.
- Roughly half the CPU/memory/disk/network sysinfo calls and DOM
  re-renders per minute, which is the actual, provable saving here — unlike
  the process-refresh change, this isn't something that needs a noisy `top`
  sample to justify.
- If resource use needs to drop further later, the same lever (poll less
  often) is the one to reach for again before looking for another single
  expensive operation to optimize away.

## Alternatives considered

- **Keep chasing individual expensive operations** (e.g. throttle disk or
  temperature refreshes too) — each one only owns a slice of a tick's total
  cost, so this is a lot of individual changes for an effect no single one
  of them can guarantee, unlike lowering the shared poll rate.
