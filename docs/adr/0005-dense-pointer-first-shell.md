# 5. The shell is dense and pointer-first

Date: 2026-11-08
Status: Superseded by 6

## Context

jyoti is a daily control surface, not a marketing page. The first Rune pass kept
a page-per-screen header, a shortcut table and sparse cards. That made the window
feel like a collection of settings pages rather than one control panel.

ReUI's application examples define the target composition: a grouped sidebar,
compact KPI tiles, flat bordered panels, a kit data table and one row per
setting. ReUI itself is React and shadcn; this repository has no JavaScript
runtime and does not install its components.

## Decision

One `SplitView` shell owns the sidebar, the window toolbar and every screen. The
sidebar groups navigation and bulbs; the content area swaps the screen body
without a second titlebar or a back stack. Dense ReUI composition rules are
reproduced with rune-kit primitives:

- KPI tiles for counts, power, brightness and sync status;
- one flat, bordered panel family with a header, body and footer;
- a compact kit table for the configured bulbs;
- label-and-control setting rows for forms, settings and help;
- `ControlSize::Small` for secondary actions and dense controls.

The app has no global keyboard shortcuts and no shortcut hints. Every action is
reachable from a visible pointer control. Standard control focus and text-entry
key handling stay because they are the input behavior of the controls
themselves, not app-wide bindings.

## Consequences

The layout is wider and taller than the old pages; the window minimum is
1080 × 720 so the four KPI tiles, the bulb table and the two-column control grid
stay legible. A future compact layout can stack the grids at a smaller width.

ReUI visual rules are translated rather than imported: colour comes from the
Rune kit, and no shadcn package enters `Cargo.toml`. The table and controls remain
native Rune components.

## Alternatives rejected

Keeping the page-per-screen chrome and shortcut table preserved two competing
navigation models and a keybinding surface the product does not need. Embedding
the React ReUI templates would add a second runtime and does not fit the native
single-process goal.
