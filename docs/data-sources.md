# Where a bar's data comes from

A status bar is, at bottom, a rendering of facts the system already knows. The
only real design question it has is *how it learns those facts*, and there are
exactly three answers: the system tells it, it reads a file, or it runs a
program and parses the output. The three are not interchangeable, and choosing
the wrong one is what makes a bar expensive, slow, or wrong.

This page settles the choice per data source, states the reasoning, and records
what was verified on a running machine rather than assumed.

## The three ways, and what each actually costs

**The system tells it.** A signal on a message bus, a socket the compositor
writes to, a file descriptor that becomes readable. The bar sleeps until
something happens and is woken with the change in hand. The cost of an idle bar
is zero; the latency of a change is the latency of the bus.

**It reads a file.** A kernel interface under `/sys` or `/proc`. There is no
process, no parsing of human prose, no daemon in between — an open and a read
of a few dozen bytes. Cheap enough that a timer around it is not the problem,
but it is still a timer: the bar decides when to look, so the value on screen
is stale by up to one interval.

**It runs a program.** A fork, an interpreter start, a program that opens its
own connections to do the same work the bar could have done, then formats an
answer as text for the bar to parse back into numbers. Every one of these steps
can fail differently, and the whole sequence repeats on every tick.

The ordering is not a matter of taste. Each step down adds a failure mode the
step above does not have, and adds it *per update*, forever.

## Why the event is not merely faster

Three arguments, and only the first is about speed.

**Correctness.** A polled bar shows a value that was true at some point in the
last interval. Between the change and the next tick it displays something false.
No interval fixes this; shortening it only narrows the window while raising the
cost. An event-driven bar is either correct or has not been told yet, and being
told is what wakes it.

**Idle cost.** A timer is a scheduled wakeup. A machine with several polled
modules never reaches a deep idle state, because something is always about to
tick. This is the mechanism behind the complaint that a status bar shows up in
the list of things draining a laptop — not the work per tick, but the fact that
ticks exist at all.

**Failure surface.** An event source that dies is noticed: the connection
breaks and the bar can reconnect and say so. A polled command that starts
failing looks exactly like a polled command whose value stopped changing. The
bar cannot tell the difference, so it shows the last good value indefinitely.

The counter-argument is honest and narrow: some facts have no event. Nothing
signals that a temperature crossed a threshold, and no package manager pushes
a notification when a mirror gains a new version. For those, a timer is not a
compromise, it is the only correct answer — and the interval should then be
chosen by how fast the value can meaningfully change, not by how fresh it would
be nice to look.

## The sources, decided

Verified on the running machine: the buses named below are live
(`org.freedesktop.UPower`, `org.freedesktop.NetworkManager`, `org.bluez`,
`org.freedesktop.UPower.PowerProfiles`), the composite battery device is
present at its guaranteed path and every property on it is annotated
`emits-change`, and `/dev/rfkill` exists and is readable by the user's group.

### Told by the system — no timer belongs here

| Fact | Source | Note |
|---|---|---|
| Battery, charge state, time remaining | the composite power device on the system bus | one object for the whole machine; every property announces its own change |
| Network state, access points, connectivity | the network daemon on the system bus | already used this way |
| Bluetooth adapters and devices | the bluetooth daemon on the system bus | the generic properties interface carries every change |
| Wireless kill switch | `/dev/rfkill`, which is readable and stays readable | a running program answers this itself; asking a command-line tool to ask the same kernel is a detour |
| Power profile | the power profile daemon on the system bus | |
| Media players | the player interface on the session bus | |
| Tray items | the item and watcher interfaces on the session bus | |
| Screen brightness | the login manager on the system bus | |
| Notifications | the notification interface the bar itself serves | |
| Workspaces, window title, keyboard layout | the compositor's own event socket | |
| Theme, wallpaper, palette | directory watches on the desktop's state directory | watch the directory, not the file: the files are replaced and relinked |

### Read from a file on the bar's own schedule

| Fact | Source | Why a timer is right |
|---|---|---|
| Processor and graphics temperature | the hardware monitoring interface under `/sys` | no event exists; the value drifts continuously |
| Processor load | `/proc` | load is an average over a window; an event is meaningless |
| Memory in use | `/proc` | same |
| Graphics load and video memory | the graphics interface under `/sys` | same |

These are open-read-close on a path resolved once. The cost is small enough
that the interval can be short when someone is looking and long when nobody is.

### Genuinely needs another program

| Fact or action | Why |
|---|---|
| Available package updates | there is no library answer that holds across distributions, and no push notification exists |
| Weather | a remote service, by definition |
| Launching what the user asked for | that is the point |
| The desktop's own scripts: theme, wallpaper, palette | the desktop owns them; the bar drives them rather than reimplementing them |
| Screen capture | the tool is the user's choice, named in the configuration |
| A command the user wrote themselves | the bar is not entitled to guess what it reads |

## What the current code does

| Service | How it learns | Verdict |
|---|---|---|
| Brightness, idle inhibitor, tray, privacy | bus | correct |
| Power, network, bluetooth, media, notifications | bus, with a timer or a spawned command alongside | correct in the main path; the leftovers below are the whole defect |
| Audio | the sound server's own library | correct |
| Clock | timer, aligned to the boundary of its period | correct — a bar showing minutes ticks on the minute |
| Temperatures, load, memory | timer over `/sys` and `/proc` | correct |
| Updates, weather | timer over a command | correct in kind |
| User-defined modules | a reading (`exec` + `interval`, re-run on a signal) or a stream (`listen_cmd`), chosen by the configuration | correct in kind; the streams are supervised as described below |

Four leftovers ask a program a question the process could answer itself:
the bluetooth service and both network backends shell out to a kill-switch
tool while already connected to the bus that reports the same state, and the
sensor layer keeps a vendor tool as a fallback for readings the kernel already
publishes.

## The shape that causes the process problem

A user-defined module written the traditional way runs as a shell loop —
`while :; do command; sleep 5; done` — that lives as long as the bar. Every
difficulty in supervising child processes follows from that one decision. A
loop that never exits has to be tracked, cancelled and cleaned up after; its
helpers are grandchildren, and most kernel guarantees do not reach a
grandchild.

A command run once, which exits on its own, needs none of it. Nothing to track,
nothing to cancel, nothing to leak. The bar already owns a schedule; the loop
adds nothing the schedule does not do better, and takes away every guarantee.

The distinction worth keeping is not "shell or no shell" but **reading versus
stream**:

* A **reading** is run, produces output, and exits. The bar decides when.
* A **stream** stays up and writes a line when something changes. It exists for
  as long as the module is on the bar.

Only a stream needs supervision. A configuration that says nothing should get a
reading, because a reading cannot leak.

## Supervising the streams that remain

| Way the bar ends | What must happen |
|---|---|
| A task is cancelled — reload, schedule change | the process group goes with it |
| The bar is signalled | the handler ends every recorded group before the process dies |
| The bar is killed outright | the next start finds the strays by their stamp and ends them |
| A stream's shell dies but its helpers do not | the helpers return to the bar, not to the service manager |

### The mechanism that looks right and is not

The kernel will send a child a signal when its parent goes away. It reads like
exactly the guarantee wanted here. It is not:

* the signal is armed against the **thread** that performed the spawn, not the
  process, so a runtime worker retiring while the bar runs on kills a healthy
  module — this is precisely how the bar lost its icons;
* it is **dropped the moment the child forks**, so it never reaches the helpers
  a shell loop starts, which are the processes that actually accumulate.

Both properties are documented kernel behaviour, not accidents of this program.

### What is used instead, and what was rejected

**Used: the subreaper request.** An orphaned descendant is reparented to the
bar rather than to the service manager, so the whole family stays inside a
subtree the bar can still see and still end. Present since Linux 3.4, therefore
available everywhere this bar runs — which is the deciding property, not
elegance.

**Rejected: a control group per spawn.** It is the only mechanism that ends an
entire tree regardless of how its members forked, and on that merit it is the
best of the three. It is rejected because creating one requires a delegated
subtree that a bar started as a plain user program cannot count on having, and
a guarantee that holds only on some installations is not a guarantee.

**Rejected: tying a child to a process descriptor.** Added in Linux 7.1, and
cleaner than either: the kernel kills the child when the bar closes the
descriptor, with no bookkeeping at all. Rejected on two counts — it covers only
the child, not its descendants, and it requires a kernel that new. Worth
revisiting when it is no longer new, and only for single processes.

The conclusion the three rejections point at is the same one the section above
reached from the other direction: the answer is not a better guard, it is fewer
processes to guard.

## Sources

- [i3status-rust — a bar built on events rather than intervals](https://docs.rs/i3status-rs)
- [Waybar issue 17 — avoid polling](https://github.com/Alexays/Waybar/issues/17)
- [Waybar issue 1320 — update modules by bus signals rather than polling](https://github.com/Alexays/Waybar/issues/1320)
- [UPower reference — the composite display device](https://upower.freedesktop.org/docs/UPower.html)
- [BlueZ — the generic properties interface and its change signal](https://www.bluez.org/page/11/)
- [The parent-death signal is per-thread and lost on fork](https://man7.org/linux/man-pages/man2/pr_set_pdeathsig.2const.html)
- [Subreaper: keeping orphaned descendants inside the supervisor](https://nv-tegra.nvidia.com/r/plugins/gitiles/linux-2.6/+/ebec18a6d3aa1e7d84aab16225e87fd25170ec2b)
- [Linux 7.1: tying a child's life to a process descriptor](https://lwn.net/Articles/1059673/)
- [Process termination in Linux, with Rust examples](https://iximiuz.com/en/posts/dealing-with-processes-termination-in-Linux/)
