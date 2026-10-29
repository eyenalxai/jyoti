# 4. Screen capture runs in a native library

Date: 2026-10-28

Status: Accepted

## Context

The screen-sync mode needs the current colour of one monitor at least once per 667 ms tick, and jyoti is one Bun process with no daemon or client/server split (ADR 1). Hyprland 0.56.2 advertises wlr-screencopy v3 and ext-image-copy-capture, and the measured cheapest path is wlr-screencopy into a `linux-dmabuf` buffer, imported as an EGLImage and downscaled on the GPU: about 0.4 ms of compositor CPU per 1440p frame, with only the small readback crossing PCIe.

The alternatives do not fit. Portal plus PipeWire adds a consent dialog, a second process and a CPU full-frame conversion, and portal-hyprland is itself built on wlr-screencopy. `ext-image-copy-capture` only delivers a frame when the screen is damaged, so it cannot guarantee a cadence. `grim` costs about 101 ms per shot, and the video tools end in an encoder. A helper process would be a daemon in all but name. Rust would add a second toolchain for no capability C lacks, and the measured harness is already C.

## Decision

Screen capture is a C11 shared library loaded into the jyoti process with `bun:ffi`. It owns one thread with its own Wayland connection, captures the selected output through wlr-screencopy v3 with `linux-dmabuf`, imports the buffer as an EGLImage, and GPU-downscales it into an RGBA grid published through a seqlock slot. Plain `copy` is used because `copy_with_damage` waits for damage and would starve on a static screen. A `wl_shm` strided-sample fallback produces the same grid when EGL import fails.

TypeScript owns all colour reduction. The `.so` only produces pixels, dimensions, a sequence number and a timestamp; the `ScreenCapture` service lists monitors, starts and stops the thread, and copies the latest grid. The library is built from source by `bun run build:native`; the `.so` and the generated protocol bindings are build artefacts, not committed files.

## Consequences

- Building jyoti for screen sync needs `cc`, `pkg-config` and `wayland-scanner`. A missing library disables screen sync and nothing else.
- The library is in-process, so a crash takes the TUI down. Its surface stays small, callback-free and free of per-frame allocation.
- v1 targets Hyprland on a single DRM device. Multi-GPU import, HDR and `ext-image-copy-capture` are later work.
- `copy` forces one compositor render per capture. At 10 Hz that is roughly 0.4% of a core in the compositor; a 60 Hz capture would buy nothing the 1.5 Hz tick can use.
- The colour algorithm and the grid size can change without recompiling C. The grid size is a start parameter.
- The colour maths stays in TypeScript, where JYO-23 can iterate on real frames without a build step.
