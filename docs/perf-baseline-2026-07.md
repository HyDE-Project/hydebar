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

## Second round — after the reaper and the optimization sweep

Same method, same machine, measured after the orphan reaper, the single
config parse, the theme cache and the rest of the sweep landed.

| Metric | First round | Second round |
| --- | --- | --- |
| Startup to mapped surface, release | 115 ms | 53 ms |
| Startup to mapped surface, debug | — | 99 ms |
| Idle CPU, release | 0.40 % | 0.50 % |
| Idle CPU, debug | 1.4 % | 1.2 % |
| CPU with a menu opening and settling, release | 0.6 % | 0.50 % |
| Resident memory, release | 136 MB | 127 MB |
| Resident memory, debug | 178 MB | 166 MB |
| Threads | 62–68 | 63 |
| Open file descriptors | ~113 | 113 |
| Zombie children | 122 | 0 |

Reading the deltas:

- Startup halved. The single TOML parse and the cached output geometry
  removed duplicated work from the launch path.
- An 8 s window containing a menu open now averages the same as idle: the
  settle animation costs less than the sampling resolution.
- Idle release moved 0.40 → 0.50 %; one scheduler tick over a 10 s window
  is 0.1 %, so this is sampling noise, not a regression.
- Memory dropped ~9 MB in both profiles — freed duplicate parses and the
  leaked defunct table, still dominated by the GPU stack.
- The zombie column is the reaper doing its job, verified live: a fresh
  bar adopted 61 strays and the count reached zero within two sweeps.

## Third round — after the motion work

Measured after the entrance wave, the per-theme sweep signatures, the
mid-screen greeting, the hover fades, the menu fade-out and the blur guard
landed.

| Metric | Second round | Third round |
| --- | --- | --- |
| Startup to mapped surface, release | 53 ms | 33 ms |
| Idle CPU, release | 0.50 % | 0.50 % |
| Resident memory, release | 127 MB | 149 MB |
| Threads | 63 | 64 |
| Open file descriptors | 113 | 114 |
| Zombie children | 0 | 0 |

- Startup dropped again because icon faces no longer resolve on the draw
  path: the first frames used to wait on fontconfig and font-file reads,
  and now they draw while a worker resolves.
- Idle is unchanged: every animation gates the frame clock, so a settled
  bar with all the new motion still wakes only for its pollers.
- Memory rose ~22 MB and holds steady. The greeting paints the
  screen-spanning menu surface at startup, so the renderer allocates that
  surface's buffers at birth instead of on the first opened menu — the
  cost moved earlier, it did not appear.
