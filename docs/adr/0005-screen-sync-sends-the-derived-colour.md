# 5. Screen sync sends the derived colour as it is

Date: 2026-10-29

Status: Accepted

## Context

Screen sync derived one colour per frame, then two rules replaced it before it reached a bulb. A frame whose 90th-percentile OKLCH chroma fell below 0.03 was judged achromatic and replaced with 4000 K. A frame that was at least 97% near-black also bypassed the reducer, the brightness envelope and the send path, holding the last colour and brightness.

Both rules served a comfort intent: keep the room warm on a white document, and avoid pumping a room through neutral on a black screen. Both broke fidelity. A solid white screen, a solid grey screen and a solid black screen all derived the same 4000 K, and a black screen sent nothing at all, so the bulbs kept whatever they showed before.

The protocol does not need the help. Colour is one integer property, `siid 2 / piid 4`, `(r << 16) | (g << 8) | b`, from `0` to `16777215`. Black is `0` and white is `16777215`; both are legal values, and the state reader already accepts `0`. "A bulb cannot emit black" describes light, not the value: a bulb asked for `0` emits no coloured light.

## Decision

Screen sync sends the derived colour as it is, including `#000000`, `#FFFFFF` and every grey between them, plus the derived brightness. The chroma judgement, the 4000 K fallback and the blackout bypass are deleted.

The sync target is exactly a colour and a brightness. Colour temperature leaves the sync target; it stays in the manual controls and in the pre-sync restore, and sync still never writes the colour-mode property. No hold, threshold, darkness mode or other special case replaces the deleted rules. The colour deadband (ΔE 0.02) and the bulb's own fade remain the only colour smoothing; brightness keeps its 900 ms scene EMA, its 8–100% envelope and its eight-point slew. Power is never written.

## Consequences

- A black screen writes `0` on the next tick and the room dims through the brightness envelope. It no longer freezes the previous scene, and the dimming takes the slew time rather than one step.
- White and grey round-trip as white and grey. No sync output is 4000 K any more.
- Achromatic frames take the same clustering path as any other frame. A fully black frame rejects the content rect and is analysed whole; every pixel is `L = 0`, so the winner is the black cluster.
- The `black` capture health, the strip's `black frame` label and the preview's blackout line are gone with the bypass. The derived colour `#000000` is the only report.
- A comfort mode for lit achromatic scenes is a new product decision, not a deferred part of this one. It must never apply to a frame with no light, and it would have to be off by default.
- Stopping a run still restores each bulb's pre-sync colour or temperature and its brightness. Nothing about the restore changes.
