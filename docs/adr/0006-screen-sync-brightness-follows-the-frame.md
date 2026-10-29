# 6. Screen sync brightness follows the frame without a slew

Date: 2026-10-29

Status: Accepted

## Context

ADR 0005 fixed the sync target to a colour and a brightness and kept the brightness rails from JYOT-24: a 900 ms scene EMA, an 8–100% envelope and an eight-point slew per 667 ms tick. The colour path dropped its own EMA in JYOT-41 and became fast: one tick to the derived colour, then the bulb's ~1 s fade. Brightness kept both low-passes and stayed slow. From a 100% scene to 30% is 70 points; at eight points per tick it takes nine ticks, about 6 s, before the last write, and the EMA alone would still take about 2.7 s to reach 95% of the change. The owner reported that a colour change feels fast and a brightness change does not.

The rails were built for comfort and photosensitivity. But the bulb's own fade is already a low-pass applied by the device; the EMA and the slew were stacked on top of it, not used instead of it.

## Decision

Scene brightness is the frame's mean per-pixel drive, unsmoothed. Sync brightness is the envelope value, written on the tick it changes: `round(clamp(8 + 92 × sceneBrightness, 8, 100))`. The eight-point slew and the applied-brightness bookkeeping are deleted. The bulb's fade is the only brightness smoothing, exactly as it is the only colour smoothing after ADR 0005 and JYOT-41.

The 8–100% envelope stays. Brightness is still written only to a bulb known to be on; power is never written; colour temperature stays out of the sync target. The flash guard keeps its constants: three targets, a two-second hold, re-armed by three same-direction targets or a peak-to-peak of 25 or less. Raw targets exposed a false hold in the old trigger: a large cut followed one tick later by a small bounce, `100, 30, 45`, is a direction flip with a three-target peak-to-peak of 70, so the guard froze brightness and reported a flashing scene on ordinary content. The trigger now also requires the reversing change itself to exceed 25 points, so `100, 8, 100` still holds and `100, 30, 45` does not.

Acceptance: with three bulbs on, a cut from solid white (100%) to a frame with mean drive 0.239 (30%) makes all three report 30 within 2 s; the visible settle follows within the bulb's fade. A cut to black writes the 8% floor in one step.

## Consequences

- A scene cut reaches the bulbs on the next tick, up to 667 ms of sampling plus the ack instead of about 6 s. The light arrives about one bulb fade after that.
- A single noisy frame can step brightness at the next tick. The flash guard catches large alternating swings; small jitter is bounded by integer rounding and one write per tick, inside the rate the transport has already carried.
- The guard no longer treats a large cut followed by a small bounce as a strobe, so a normal scene change does not freeze brightness for two seconds or show `flashing scene`.
- A cut to black dims in one write, through the bulb's fade, rather than through the slew. ADR 0005's consequence that "the dimming takes the slew time rather than one step" no longer holds.
- A start on a darker scene no longer slews from the pre-sync brightness; the first tick writes the scene's value, as colour already did. Stop still restores.
- The flash guard protects brightness only. Colour keeps updating through a hold, by design since JYOT-38; this ADR does not change that.
- `ReducerState` keeps only the warm-start centroids. `SCENE_TAU_MS`, `smoothing`, `resetSmoothing`, `BRIGHTNESS_SLEW`, `slewBrightness` and `appliedBrightness` are gone.
