# 9. The capture thread owns the sync run

Date: 2026-11-09
Status: Accepted

## Context

Screen sync ticks at 1.5 Hz and publishes a colour and brightness to every
target bulb. The tick used to live in `AppSpec::poll`, which reads the captured
scene and sends the worker commands. Rune stops polling while the app window is
on a hidden workspace; the capture thread keeps producing frames, but the poll
never drains them. A run then publishes only when the window is shown again,
and the bulb jumps to the latest queued colour at that moment.

## Decision

The capture thread owns the sync run. On start it receives the gain, the colour
intensity and a list of targets with their worker senders and on-state. On each
frame it applies the intensity, the gain, the flash guard and the colour
deadband, sends the bulb commands directly, and emits the published output for
the UI.

`AppSpec::poll` configures the run and mirrors the published events into the
Sync page. It no longer decides what to send. Stop sends a command to the
thread; the thread reports the bulbs whose brightness it drove, and the UI
feeds that set into the restore report.

## Alternatives

- Keep the publish decision in `poll` and keep the event loop awake with a
  timer: rejected because a one-second, fifty-millisecond and zero-delay timer
  all stop waking the hidden window on Hyprland.
- Move the whole run into a separate thread: rejected because the capture
  thread already owns the tick, the reducer state and the selected monitor.

## Consequences

Sync keeps driving the bulbs while the window is hidden, which is the normal
case when the user watches the content on another workspace. The Sync page can
lag until the window returns; it then takes the latest published frame. The
screen module now depends on the bulb worker command type, and restore waits
for the thread's drove report before it sends the restore commands.
