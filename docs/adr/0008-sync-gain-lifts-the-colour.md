# 8. The sync gain lifts the colour, not only the brightness property

Date: 2026-10-29

Status: Withdrawn. The gain turned greys and pale colours toward white; it was reverted by JYOT-52. The brightness-only gain is recorded in ADR 0009.

## Context

ADR 0007 gave screen sync a relative brightness gain. It scales the rounded 8–100% envelope result and writes that number to the bulb's brightness property. A live read-back with three bulbs showed what that means at the limit: at 1× the bulbs read brightness 57% with colour `#9D9190`; at 2× they read brightness 100% with the same colour `#9D9190`. The owner still found the room too dim.

The numbers explain it. The brightness property was already at its maximum. The derived colour was a mid grey, and a grey at 100% emits roughly a third of the light of white at 100%. Brightness cannot brighten the colour: it scales the emitter, not the colour it emits. ADR 0005 sends the derived colour as it is, so the only remaining lever is the colour's own lightness.

## Decision

The sync gain scales the derived colour's OKLCH lightness as well as the brightness property, before the colour is packed and sent:

- `lightness = clamp(derived lightness × gain, 0, 100)`, with hue preserved and chroma fitted to the sRGB gamut by the existing conversion.
- At 1× the derived colour is returned unchanged; ADR 0005's fidelity rule still holds for the default.
- Above 1× a grey lifts toward white and a saturated colour lightens until it clips; below 1× the colour darkens.
- Black stays black at every gain. A frame with no light is still sent as black; the brightness property follows the envelope separately.
- The strip's swatch is the gained target colour, so the swatch and the next write carry the same value.
- No new key: the existing `+` and `-` gain adjusts both. The gain keeps its 0.5–2.0 range and 1.0 default from ADR 0007; its lifetime is the preference file from ADR 0009.

## Consequences

- A mid grey at 2× becomes white at 100% brightness. The room gets the light the brightness property alone could not produce.
- Bright scenes clip at white and saturated colours lose chroma as they approach the gamut edge; this is the intended trade when the owner raises the gain.
- At 1× the output is exactly the derived colour, so the default remains faithful.
- The colour deadband compares gained colours. A gain change above the deadband republishes on the next tick; a small step on a very dark colour can stay under ΔE 0.02, so only its brightness moves. The bulb's fade still smooths what is written.
- The strip's swatch is the gained target, so it matches the colour the next write would send. The existing colour deadband can still leave the last written colour within ΔE 0.02 of it, as it did before the gain.
- Brightness-only bulbs and bulbs with unknown power keep the existing rules: colour is still written, brightness only to known-on bulbs.
- The stop restore, flash guard, tick interval and power rules are unchanged.
- ADR 0007's gain now covers brightness and colour. This ADR extends it; it does not supersede the envelope or the gain's range.
