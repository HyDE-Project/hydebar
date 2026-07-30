# Performance baseline — July 2026

Measured on the development machine (3840×2160 output, Hyprland, release
build unless stated) so later changes have something honest to be compared
against. Method: `/proc` sampling over 10 s windows; surface-mapping time
taken from launch to the compositor listing the bar's layer.

| Metric | Value |
| --- | --- |
| Startup to mapped surface | 115 ms |
| Idle CPU, release | 0.40 % |
| Idle CPU, debug | 1.4 % |
| CPU with a menu opening and settling | 0.6 % avg over 8 s |
| Resident memory, release | 136 MB |
| Resident memory, debug | 178 MB |
| Threads | 62–68 |
| Open file descriptors | ~113 |

## What the measurement found and what was done

The scan found **122 zombie processes**, all children of the running bar:
gpg helpers spawned indirectly by the update check, double-forked away from
their parents, adopted by the bar (it claims orphans on purpose, so its
sweep can end strays) and then never buried. Fixed in the same commit as
this file: a reaper thread wakes once a minute and buries only the adopted
children that stayed dead across two consecutive looks — the bar's own
spawns all have a waiter and never survive one look, so nothing is stolen
from a live wait.

## Reading the numbers

- The idle figures confirm the frame-clock gating: a settled bar consumes
  well under one percent and wakes only for its pollers.
- Memory is dominated by the GPU stack (wgpu over Vulkan); the number is
  the price of the renderer, not of the modules. If it ever matters on a
  small machine, the software renderer feature is the lever to try first.
- Kernel perf counters are locked down on this machine (`perf` absent,
  `perf_event_paranoid = 2`), so no flamegraph; the `/proc` deltas above
  are the honest substitute. If a flamegraph is ever needed, build with
  frame pointers and run `perf` as root, or use `tracy`-style in-process
  instrumentation behind a feature flag.

## Levers already identified for the next round

- Software renderer comparison run (memory-driven).
- In-process frame timing behind a debug feature, so animation work has a
  budget check instead of an eyeball check.
