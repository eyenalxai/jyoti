# 9. The sync gain is remembered and visible

Date: 2026-10-29

Status: Accepted

## Context

ADR 0007 made the sync gain session-only with a default of 1.0. In use the owner maxed it on every run, and the control was only shown in the strip when it was not 1.0, so a fresh launch looked unchanged and the control was easy to miss.

A colour-lift attempt was tried first: the gain also scaled the derived colour's lightness. It turned greys toward white and washed coloured content out, which is not what the owner meant by brighter. It was reverted; the gain is brightness-only again.

## Decision

- **Remember the gain.** The gain is stored in `$XDG_CONFIG_HOME/jyoti/prefs.json`, next to `bulbs.json`, as `{ "version": 1, "syncGain": 1.5 }`. The sync engine loads it at start, saves it on every gain change, clamps it to 0.5–2.0 and rounds it to 0.1. A first run starts at 1.0. A missing or unreadable file falls back to 1.0; a failed save leaves the gain applied for the session and surfaces as a toast.
- **Show the gain always.** The strip renders `57% ×1.0` at the default and `57% ×1.5` after a press while syncing; the paused form is `45% · gain ×1.0` because the brightness is the last one written. The sync-on toast appends `+/- gain`.
- **Leave the colour alone.** The gain scales only the rounded envelope result. The derived colour is sent as it is at every gain, so a press never whitens or desaturates it.

## Consequences

- The gain survives launches and is shared by every bulb in the run. Deleting `prefs.json` resets it to 1.0.
- The preference file is written atomically with mode 0600 and never holds a token. The bulb list stays in `bulbs.json`.
- A save failure does not roll back the in-memory gain; the toast says the preference was not saved and the next press retries.
- A gain change repaints brightness, which the strip already shows. No colour write is forced; the colour deadband still filters frame noise.
- ADR 0007's session-only lifetime is superseded; its range, step and default are unchanged. The colour lift is not part of this decision; a white-mix or saturation control would be a new one.
