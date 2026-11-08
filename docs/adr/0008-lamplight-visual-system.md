# 8. The visual system is Lamplight

Date: 2026-11-08
Status: Accepted

## Context

The panel is a daily utility used beside an editor, often at night. Its window
is deliberately compact, so every vertical point is scarce and the light should
carry more weight than the surrounding chrome. A stock neutral theme with a
system accent made the light one card among several, left the lower window
empty, hid readable type behind bold control labels and read as an unmodified
component gallery. Light mode also sat close enough to the panel colours that
structure disappeared.

The window size, single-pane flow and pointer-first rule of ADR 6 still hold.
This decision replaces the visual treatment built on them.

## Decision

The visual system is Lamplight, derived from the product rather than from a
generic kit:

- **Colour.** Warm paper in light, espresso in dark, one brass accent for
  interaction, light-state colours for status. All values are semantic tokens in
  `src/ui/theme.rs`; screens never name a raw colour.
- **Type.** Fraunces carries page titles, the light name and the colour hex;
  Inter carries the interface and tabular figures; IBM Plex Mono carries
  addresses, hex input and paths. The three families are bundled with their SIL
  Open Font License texts and loaded through `AppSpec::fonts`. No text role is
  smaller than 12 points.
- **Light stage.** Control opens on a field filled with the light the bulbs are
  showing. A cream nameplate on the field carries the state, name and brightness
  so the readout stays legible over any colour. The stage is a readout, not a
  control; every action stays on a visible pointer control.
- **One deck.** Power, brightness, mode and warmth or colour sit in one panel of
  hairline-separated rows. A screen sync run dims the rows it owns and leaves
  power available.
- **Quiet navigation.** The three sections are text tabs marked by a brass rule
  under the active label. The filled accent pill is gone.
- **Shared surfaces.** Sync uses the same stage and nameplate for its captured
  colour. Settings uses group captions over one surface per group. Forms use one
  surface with a footer; Colour uses a paint-chip preview over the tuning deck.
  Decorative badges and duplicate panel headers are removed.
- **Accessibility.** Live figures use tabular numerals. Focus rings are neutral
  ink, not the accent. Stage text sits on the nameplate or uses contrast-aware
  ink. Interactive targets are 40 points where the kit allows, and scroll
  viewports carry the kit scroll bar.

## Consequences

The bundle grows by about 1.3 MB for the three font files. The theme is owned by
the app rather than inherited from the kit, so new components must use its
tokens. The light stage takes enough height that Control and Settings scroll at
the 480 × 600 minimum; at the default 500 × 680 every Control row and the full
Settings list fit.

The composition rules remain inspired by ReUI, but no React and no shadcn
package enters the crate. Fonts and colours are the only new assets.

## Alternatives rejected

- Keeping the neutral kit with a different accent: the light would still be a
  card among cards and the window would still be half empty.
- A cool measuring-equipment theme: precise, but interchangeable with any
  utility and disconnected from a product about light.
- A dark-only theme: the panel follows the desktop, and light mode is a real
  appearance rather than a translation.
- Shipping no fonts: system fallbacks change the display voice and the width of
  live numbers between machines.
