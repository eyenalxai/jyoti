# 9. The sync gain is remembered, always visible and repaints on a press

Date: 2026-10-29

Status: Accepted

## Context

ADR 0007 made the sync gain session-only with a default of 1.0. In use the owner maxed it on every run, and the control was only shown in the strip when it was not 1.0, so a fresh launch looked unchanged and the control was easy to miss. The colour deadband also swallowed small gain steps on very dark colours: the brightness moved but the colour did not, so a press could look dead.

ADR 0008 fixed the light output by lifting the colour as well. This ADR fixes the control around it: the setting has to survive a launch, be visible at the default, and react to every press.

## Decision

- **Remember the gain.** The gain is stored in `$XDG_CONFIG_HOME/jyoti/prefs.json`, next to `bulbs.json`, as `{ "version": 1, "syncGain": 1.5 }`. The sync engine loads it at start, saves it on every gain change, clamps it to 0.5–2.0 and rounds it to 0.1. A first run starts at 1.0. A missing or unreadable file falls back to 1.0; a failed save leaves the gain applied for the session and surfaces as a toast.
- **Show the gain always.** The strip renders `57% ×1.0` at the default and `57% ×1.5` after a press, in both the syncing and paused forms. The sync-on toast appends `+/- gain`.
- **Repaint on a press.** A tick whose gain differs from the previous tick forces a colour write, bypassing the ΔE 0.02 deadband. A device rejection still suppresses the write. Paused changes are forced on the first tick after resume.

## Consequences

- The gain survives launches and is shared by every bulb in the run. Deleting `prefs.json` resets it to 1.0.
- The preference file is written atomically with mode 0600 and never holds a token. The bulb list stays in `bulbs.json`.
- Every press is visible even on dark colours; each press can add one colour write per bulb. The deadband still filters frame noise, not key presses.
- A save failure does not roll back the in-memory gain; the toast says the preference was not saved and the next press retries.
- A first run still sends the derived colour unchanged at 1.0, so ADR 0005's fidelity rule holds until the owner raises the gain.
- ADR 0007's session-only lifetime is superseded; its range, step and default are unchanged.
