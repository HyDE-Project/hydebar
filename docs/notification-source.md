# Choosing who draws the notifications

The bar offers three answers: its own popups, the compositor's native ones, or
whatever daemon the session already runs. The setting looks like a preference
and is in fact a question about an exclusive system resource, which is why a
naive implementation half-works.

## The one fact everything follows from

`org.freedesktop.Notifications` is a well-known bus name, and a well-known name
has exactly one owner. Only one notification server can exist in a session; two
daemons cannot split the work, and the second one to ask does not get a turn.
Which one wins is decided by whoever asked first.

That single constraint settles the shape of all three modes:

| Mode | What it means mechanically |
|---|---|
| The bar's own popups | the bar **is** the notification server and paints the popups itself |
| The compositor's native ones | the bar **is** the notification server and forwards each notification to the compositor's own front end |
| The session's daemon | the bar is **not** the server and does not ask for the name at all |

The first two are the same act with different painting. This is also how the
existing compositor-notification bridges are built: they are notification
servers that translate each call into a compositor instruction. There is no
mechanism that lets a program paint freedesktop notifications without holding
the name.

## Why asking for the name is not enough

A name request can carry two flags: one saying *replace whoever holds this*,
and one saying *I am willing to be replaced later*. The first only works when
the current owner set the second. The daemons a session normally starts do not,
so the request does not fail — it is **queued**, and the queue only advances
when the incumbent exits.

That failure is silent, which is what makes it dangerous: the setting says the
bar draws the popups, the bar believes it asked, and the old daemon keeps
painting. The bar must therefore read the reply and treat *queued* as failure,
not as success.

## Why the bar must not stop the incumbent

The tempting fix is to stop whatever holds the name and take it. It is wrong,
and on this machine it is actively destructive.

Verified here: there is no notification unit and no notification D-Bus service
file at all. The daemon is started by the desktop session as a plain child
process. Asking the service manager which unit its process belongs to therefore
does not name a notification service — it names the **session application unit
that happens to contain it**, which also contains the user's terminal. Stopping
that unit stops the terminal. That is not a hypothetical: it happened.

The general rule this settles: a bar may end processes it started itself, and
nothing else. A daemon started by the session belongs to the session.

## What the reliable, automatic design actually is

The choice is a session-level decision, and the only place it can be made
without a fight is where the session decides what to start.

1. **The bar asks for the name and reads the reply.** Primary owner means the
   mode is live. Queued means another server holds it; the bar says so, naming
   the program and its process, and keeps drawing nothing rather than
   pretending.
2. **The bar ships a D-Bus service file.** With no daemon running, the first
   notification activates the bar, and the choice needs no session edit at all.
   This is the mechanism the specification provides for exactly this question,
   and a file in the user's own data directory outranks the system one.
3. **The installer, not the running bar, settles a session that already starts
   a daemon.** Choosing the bar's popups means the session should stop starting
   the other one. That is an edit to session startup, made once, with the user's
   knowledge — not a process killed behind their back every time the bar comes
   up.
4. **Switching modes at runtime is honest about what it can do.** Between the
   bar's own popups and the compositor's, nothing external changes: the bar
   holds the name either way and only the painting differs, so the switch is
   instant and always works. Switching to the session's daemon releases the
   name. Switching *away from* a session daemon that is already running cannot
   succeed until that daemon stops, and the bar says so instead of failing
   quietly.

## What the bar owes the specification once it holds the name

Holding the name is a promise, and a server that keeps it badly is worse than
no server: applications get no error, they simply lose their notifications.

| Obligation | What it means here |
|---|---|
| The interface at its object path | four methods: report capabilities, accept a notification, close one, describe the server |
| Honest capabilities | report only what is actually drawn, since applications choose what to send from this answer |
| Both signals | say when a notification closed and why, and say when the user invoked an action; a sender that never hears back cannot clean up |
| Expiry rules | a timeout of minus one means the server decides, zero means it never expires, anything else is milliseconds |
| Urgency | three levels, and the specification is explicit that a critical notification should not expire on its own |

That last line is also the rule the bar broke in the other direction: it sent
its own notices at critical urgency, and the daemon dutifully kept them on
screen until dismissed by hand. The urgency of a message is a promise about how
important it is, not a way to make it noticeable.

## Why this cannot be settled by packaging alone

Several daemons ship an activation file, and installing two of them leaves the
choice to whichever the bus picks — the decision leaves the user's hands
entirely. That is the failure the whole mechanism has, and it is the reason the
larger desktops stopped relying on activation and start their notification
server as part of session startup instead.

The conclusion for a bar that wants this to work by itself: ship the activation
file so a session with no daemon needs no configuration at all, and make the
session-level choice explicit at install time for a session that already has
one. Never at runtime, and never by ending someone else's process.

## What is wrong in the bar today

| Symptom | Cause |
|---|---|
| The setting says the bar's own popups, the daemon's appear | the queued reply was read as success |
| A refused action showed a compositor bubble regardless of the setting | the bar's own notices bypassed the chosen source |
| A notice stayed on screen until dismissed by hand | it was sent at critical urgency, which a daemon is entitled to keep up forever |

The first two are fixed by reading the reply and by routing the bar's own
notices through the chosen source; the third by sending at ordinary urgency
with an explicit lifetime. None of them requires touching another process.

## Sources

- [Desktop notifications — only one daemon can own the name](https://bbs.archlinux.org/viewtopic.php?id=289214)
- [Resolving desktop notification D-Bus service conflicts](https://kevinlocke.name/bits/2020/04/12/resolving-desktop-notifications-dbus-service-conflicts/)
- [A notification server that forwards to the compositor's own notifications](https://github.com/codelif/hyprnotify)
- [D-Bus activation is a race between providers claiming the interface](https://github.com/linuxmint/cinnamon/issues/7824)
