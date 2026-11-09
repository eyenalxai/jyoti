# 6. The window is compact and single-pane

Date: 2026-11-08
Status: Accepted (window and flow retained; visual treatment and metrics superseded by 8)

## Context

The dense shell filled a large window with a sidebar, a configured-bulb table,
KPI tiles and several panels. The same bulb could be selected from the sidebar
or the table, and screen sync was reachable from the sidebar, a card and the
toolbar. A small bulb set does not provide enough information for that density,
so the result repeated controls instead of becoming easier to use.

ReUI’s compact application patterns favour one surface per job: a picker in the
header, a short action menu, tabs for sections, and settings arranged as short
rows. The window is a daily utility, not a dashboard that fills a desk display.

## Decision

The app opens in a 500 × 680 window with a 480 × 600 minimum and no maximum.
The page is capped at 484 points and centered when the window grows. One `Page` frame holds the window title, one three-tab bar and the active
screen:

- **Control** owns the bulb selector, one light surface with a live preview,
  brightness, mode and warmth or colour, and an overflow menu for refresh, edit
  and remove.
- **Sync** owns capture status, monitor selection, start/pause/stop, brightness
  gain and colour intensity. A stop reports its restore outcome in a toast.
- **Settings** owns appearance, storage and about.

The bulb selector is a dropdown menu with one row per bulb; there is no sidebar
and no bulb table. The active section uses a tinted accent pill so it reads
against the page rather than blending into it.
A missing bulb set shows one compact empty panel with one Add action. Add, edit,
remove, refresh, colour and screen sync each have one entry point. Forms and the
remove confirmation sit below the tabs and carry a Back control. The colour
editor expands inside the Control deck; it is not a page and has no Back
control. The colour editor is a live preview with hue, saturation and brightness
gradients, not a channel form.

The readable scale starts at 13 points for captions and 14 for control text.
Tabs are 40 points high, field triggers 36, primary buttons and sliders 28. The
window opens 40 points taller so a form footer stays in view.

The app has no global keyboard shortcuts. Native control focus and text entry
remain because they are the behavior of the controls themselves.

## Consequences

Less content is visible at once, so forms and the full storage list scroll. The
window fits beside an editor and reads as a utility rather than a workspace. The
overflow menu hides secondary bulb actions, which is acceptable for a small set
of frequent actions. The compact layout leaves no room for a KPI row or a second
selection surface without bringing back duplication.

ReUI remains a source of composition rules, not a runtime dependency. Panels
and menus use rune-kit components; the compact tab bar and gradient sliders are
app-specific wrappers over Rune primitives, not a second component system. No
React or shadcn package enters the crate.

## Alternatives rejected

Keeping the sidebar, table and KPI row preserved three ways to see the same small
set of bulbs. Adding links between screens increased reachability but also
created the duplicate entry points this change removes. A fixed narrow page with
a sidebar would still spend a third of the width on a list that the dropdown can
carry in one row.
