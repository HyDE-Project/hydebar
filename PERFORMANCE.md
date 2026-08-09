# Performance

This document tracks hydebar performance metrics and optimization efforts.
The measured record lives in
[docs/perf-baseline-2026-07.md](docs/perf-baseline-2026-07.md); this page
summarises it and keeps the methodology.

## Goals

- **CPU usage:** < 1% idle; a menu opening should not be visible in the numbers
- **Startup:** well under 100ms to a mapped surface
- **Zombie/stray children:** zero
- **Memory:** bounded by the GPU stack, not by the modules

## Current Numbers (July 2026, release build, 4K output)

| Metric | First round | After the optimization sweep |
|--------|-------------|------------------------------|
| Startup to mapped surface | 115 ms | 53 ms |
| Idle CPU | 0.40 % | 0.50 % (sampling noise) |
| CPU with a menu opening and settling | 0.6 % | 0.50 % |
| Resident memory | 136 MB | 127 MB |
| Threads | 62–68 | 63 |
| Open file descriptors | ~113 | 113 |
| Zombie children | 122 | 0 |

What the sweep changed:

- **Single config parse and cached output geometry** — halved startup.
- **Orphan reaper** — the bar claims orphaned descendants on purpose (so it can
  end strays from earlier runs); a reaper thread now buries the adopted
  children that stay dead across two consecutive looks. Verified live: 61
  adopted strays reached zero within two sweeps.
- **Frame-clock gating** — a settled bar wakes only for its pollers; the menu
  settle animation costs less than the sampling resolution.

Reading the memory number: it is dominated by the GPU rendering stack (wgpu
over Vulkan). It is the price of the renderer, not of the modules; the
software renderer is the lever to try first if it ever matters on a small
machine.

## Architectural choices that keep the bar cheap

- **Events over polling.** Every source that can push does push: D-Bus
  signals, the Hyprland event socket, file watches. Timers exist only for
  facts that have no event (temperatures, load, memory). The full decision
  record is in [docs/data-sources.md](docs/data-sources.md).
- **Redraw coalescing.** The event queue coalesces at enqueue time — a
  snapshot replaces its stale twin, a duplicate redraw folds into the tail —
  so a noisy producer repaints the bar once, not once per event. The first
  event of a burst is delivered without a grace window: a user click pays no
  batching latency.
- **Registration gating.** A module absent from the layout starts no
  background work at all (`gui/src/app/update/registration.rs`).
- **Bounded runtime.** The tokio pool is pinned to 4 workers
  (`HYDEBAR_RUNTIME_THREADS` overrides it) instead of one per CPU, since the
  workload is parking on D-Bus, Wayland, Hyprland and child-process pipes.
- **Arc<Config>.** Hot reload clones a pointer, not the config tree.

## Levers identified for the next round

- Software renderer comparison run (memory-driven).
- In-process frame timing behind a debug feature, so animation work has a
  budget check instead of an eyeball check.

## Methodology

Kernel perf counters are locked down on the measurement machine
(`perf_event_paranoid = 2`), so the numbers come from `/proc` sampling over
10 s windows; surface-mapping time is taken from launch to the compositor
listing the bar's layer.

### Binary Size
```bash
cargo build --release
ls -lh target/release/hydebar-app  # Unstripped
strip --strip-all target/release/hydebar-app -o hydebar-app-stripped
ls -lh hydebar-app-stripped  # Stripped
```

### Memory Profiling
```bash
heaptrack ./target/release/hydebar-app
heaptrack_gui heaptrack.hydebar-app.*.gz
```

### CPU Profiling
```bash
perf record --call-graph dwarf ./target/release/hydebar-app
perf report
```
(Requires a machine where `perf` is available and permitted; otherwise sample
`/proc/<pid>/stat` deltas.)

### Startup Time
```bash
hyperfine --warmup 3 './target/release/hydebar-app'
```

## Resources

- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [cargo-flamegraph](https://github.com/flamegraph-rs/flamegraph)
- [heaptrack](https://github.com/KDE/heaptrack)
- [perf](https://perf.wiki.kernel.org/)
