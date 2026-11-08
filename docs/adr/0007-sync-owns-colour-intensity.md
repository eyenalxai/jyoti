# 7. Sync owns colour intensity

Date: 2026-11-08
Status: Accepted

## Context

The colour picker already exposes saturation for a manual RGB colour. Screen
sync derives a colour from the screen, and derived colours are often muted. A
second saturation control on Control duplicated the picker and could not change
what sync sent.

## Decision

Sync has one persisted colour-intensity value from 0% to 100%. It runs after the
reducer and before the colour deadband and publish:

- 0% sends the derived colour unchanged;
- 100% raises its HSV saturation to full;
- hue and HSV value stay put.

Manual colours continue to use the picker's saturation slider. The intensity
does not touch brightness, the gain, the flash guard or restore. Changing it
while a run is live marks the next frame for publish.

## Consequences

The sync output is predictable and survives launches in `prefs.json`. A muted
screen colour can be made vivid without changing its light level. At 100% the
output can be much more saturated than the source; 0% is an exact source pass.
