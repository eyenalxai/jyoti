# 7. Screen sync brightness has a relative gain

Date: 2026-10-29

Status: Accepted

## Context

ADR 0005 fixed the sync target to a colour and a brightness. ADR 0006 made scene brightness the frame's unsmoothed drive and kept the 8–100% envelope: `round(clamp(8 + 92 × sceneBrightness, 8, 100))`. The owner reports that synced bulbs are not bright enough on ordinary content and asks for control over brightness relative to the derived value. The manual brightness control does not help: sync owns colour, brightness and temperature while a run exists, and an absolute percentage is a different concept from a multiplier on whatever the frame produced. What is missing is a way to shift the whole frame-following mapping up or down without leaving sync.

## Decision

Sync brightness takes a gain:

`round(clamp(syncBrightnessFor(sceneBrightness) × gain, 8, 100))`

The existing envelope is rounded to a whole percentage before the gain is applied, so the strip's number is the value that was scaled.

- The gain is 0.5 to 2.0 in 0.1 steps and defaults to 1.0.
- It scales the envelope's output, not the scene and not the clamp. The 8% floor is a floor on the result: at a gain below 1 a dark scene stays at 8%, and at a gain above 1 the floor rises with everything else.
- The result stays clamped at 100%, so a bright scene cannot exceed the bulb's maximum.
- The flash guard sees the gained target. Its window, hold and thresholds do not change.

The gain lives in the sync engine, not the config file. It survives stop and start within one jyoti process and resets to 1.0 on launch, like the captured monitor choice. `+` and `-` adjust it while a run exists, syncing or paused; `=` acts as `+` so the unshifted key works on layouts where plus needs Shift. The strip shows `68% ×1.5` while the gain is not 1.0, the run keys name the control, and the help screen lists it. While paused, the gain updates the strip but writes nothing; because the shown brightness is the last one written, the strip separates them as `45% · gain ×1.5`, and the next tick after resume applies the gain.

## Consequences

- A 45% envelope result at 1.5× writes 68%. A 100% result at 0.5× writes 50%, and at 2× stays clamped at 100%.
- A gain below 1 dims, bounded by the 8% floor. A gain above 1 lifts the floor as well: 8% becomes 12% at 1.5×. This follows from scaling the displayed value rather than the scene.
- The gain applies on the next tick, so the bulb's own fade remains the only smoothing. The tick interval, one write per bulb per tick, the power rules and the restore are unchanged.
- A gain change alone is a one-direction step, so it does not trip the flash guard. The guard still acts on the gained targets exactly as it acted on the raw ones.
- The gain is not persisted and is not adjustable while sync is idle; a new process starts at 1.0.
- ADR 0006's envelope remains the base mapping. This ADR adds the gain around it and does not supersede it.
