# 1. The app is a Rune desktop app

Date: 2026-11-08
Status: Accepted

## Context

The product needs a native window, pointer-first controls and a maintainable
implementation that can reach macOS and Windows later. The previous interface
was a terminal UI. Rune already provides the window, layout, input, text, GPU
renderer and headless snapshot tooling, so the app does not need to build a UI
toolkit first.

## Decision

jyoti is one Rust binary on Rune. The window is a `rune-kit` page with a section
tab bar and the active screen. Views are plain Rust functions over an app state.
The Rune dependency is pinned to a `dev` rev; the app does not fork it.

There is one main window. Screen sync, settings, add, edit, remove and the
colour picker are pages in that window, not separate windows or a tray panel.

## Alternatives

- Keep the terminal UI: rejected because the product needs a native window and
  pointer input.
- Build a UI toolkit in the app: rejected because Rune already supplies the
  renderer, layout, text and headless tooling.
- A tray-first or multi-window shell: rejected for the first release because the
  main window is the product; a tray can be added later without reshaping the
  pages.

## Consequences

The app depends on a pre-release framework pinned by revision. A Rune change may
require a dependency bump. The headless snapshot and tree tooling is available
for verification.
