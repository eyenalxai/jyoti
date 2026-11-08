# 2. Screen capture uses libwayshot with the GPU path

Date: 2026-11-08
Status: Accepted

## Context

Screen sync reads one monitor at 1.5 Hz and reduces a frame to a small colour
grid. The full frame is several megabytes; doing the whole reduction on the CPU
adds avoidable memory traffic and a second copy. The app targets Linux Wayland
first, with macOS and Windows later.

## Decision

`libwayshot` 0.10 is the first `Capture` backend. It wraps `wlr-screencopy` and
`ext-image-copy-capture`, imports the compositor's DMA-BUF into an EGLImage and
binds it to a GL texture. The app blits that texture into a 64×36 framebuffer
and reads only the reduced grid back. The CPU reducer sees 2,304 pixels, not a
full monitor.

Capture sits behind a `Capture` boundary. A ScreenCast portal/PipeWire backend
can cover GNOME and KDE later, and macOS and Windows can add their own backends
without changing the sync reducer or the UI.

## Alternatives

- CPU shared-memory capture: rejected because the owner does not want the
  full-frame reduction on the CPU.
- Keep the C capture library behind FFI: rejected because the app logic is Rust
  and `libwayshot` already wraps the same protocols.
- Portal plus PipeWire first: rejected for the first release because it adds a
  consent dialog and a larger consumer, and the target compositor exposes
  `wlr-screencopy`.
- `scap` or `xcap` as the primary crate: rejected because `scap` is a beta with
  a year-old release and `xcap` returns CPU images.

## Consequences

Linux capture works on wlroots compositors (Hyprland, Sway, River). GNOME and
KDE need the future portal backend. `libwayshot` is pre-1.0, so the dependency is
pinned exactly and the capture boundary keeps a backend swap contained.
