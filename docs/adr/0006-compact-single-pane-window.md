# 6. The window is compact and single-pane

Date: 2026-11-08
Status: Accepted

## Context

The dense shell filled a large window with a sidebar, a configured-bulb table,
KPI tiles and several panels. The same bulb could be selected from the sidebar
or the table, and screen sync was reachable from the sidebar, a card and the
toolbar. A small bulb set does not provide enough information for that density,
so the result repeated controls instead of becoming easier to use.

ReUI's compact application patterns favour one surface per job: a picker in the
header, a short action menu, tabs for sections, and settings arranged as short
rows. The window is a daily utility, not a dashboard that fills a desk display.

## Decision

The app opens in a 500 × 640 window with a 480 × 600 minimum. One `Page` frame
holds the window title, one four-tab bar and the active screen:

- **Control** owns the bulb selector, light controls, power switch, and an
  overflow menu for refresh, edit and remove.
- **Sync** owns capture status, monitor selection, start/pause/stop, gain and the
  restore report.
- **Settings** owns appearance, storage and about.
- **Help** is one accordion.

The bulb selector is a dropdown menu with one row per bulb; there is no sidebar
and no bulb table.
A missing bulb set shows one compact empty panel with one Add action. Add, edit,
remove, refresh, colour and screen sync each have one entry point. Forms, the
colour picker and remove confirmation replace the tabs and carry a Back control.

The app has no global keyboard shortcuts. Native control focus and text entry
remain because they are the behavior of the controls themselves.

## Consequences

Less content is visible at once, so forms and the full storage list scroll. The
window fits beside an editor and reads as a utility rather than a workspace. The
overflow menu hides secondary bulb actions, which is acceptable for a small set
of frequent actions. The compact layout leaves no room for a KPI row or a second
selection surface without bringing back duplication.

ReUI remains a source of composition rules, not a runtime dependency. Panels,
selects, tabs and menus are rune-kit components; no React or shadcn package
enters the crate.

## Alternatives rejected

Keeping the sidebar, table and KPI row preserved three ways to see the same small
set of bulbs. Adding links between screens increased reachability but also
created the duplicate entry points this change removes. A fixed narrow page with
a sidebar would still spend a third of the width on a list that the dropdown can
carry in one row.
