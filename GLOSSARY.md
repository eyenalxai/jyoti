# Glossary

## Bulb

A Xiaomi light on the local network. A bulb record has an ID, a display name, an
IP address and an optional model. The token is not part of the record; it lives
in the keyring. There is no fixed count and no numeric index.

## Bulb ID

The stable identifier jyoti generates when a bulb is added. It is a UUID stored
in the config file and used as the keyring account name.

## Keyring

The operating system's secure credential store, reached through the `keyring`
crate: Secret Service on Linux, Keychain on macOS and Credential Manager on
Windows. It holds one token per bulb under the `jyoti` service. The keyring is
the only place a token is stored.

## Config file

`$XDG_CONFIG_HOME/jyoti/bulbs.json`, falling back to
`~/.config/jyoti/bulbs.json`. It holds the bulb list's metadata and a format
version. It never holds a token.

## Preferences file

`$XDG_CONFIG_HOME/jyoti/prefs.json`, next to the config file. It holds
preferences that survive launches, currently the sync brightness gain. It never
holds a token.

## Token

The 16-byte device key, written as 32 hex characters. It authenticates and
encrypts every miIO packet. It is stored in the keyring, never in the config
file, never committed and never logged.

## Device ID

The four-byte identifier a bulb returns in its hello reply. The next request
echoes it so the bulb can associate the reply with the request.

## miIO

Xiaomi's UDP protocol on port 54321. A request sends a 32-byte hello, receives
the device ID and stamp, then sends an AES-encrypted JSON command. `bulbs.py` is
the reference implementation.

## All bulbs

A selection that stands for every configured bulb at once. It is not a bulb: it
has no ID, address, token or stored state, so edit and remove do not act on it.
The row appears first in the bulb rail while two or more bulbs exist, and it is
the default selection on first load. Its state is the aggregate of the bulbs:
power is on, off or mixed; quality is White or Colour when every reachable bulb
agrees and mixed when they do not. When the bulbs disagree on the mode the mode
switch shows mixed and no mode's settings are drawn; choosing White or Colour
applies that mode to every bulb. Power uses a quorum: if every reachable bulb is
on, one toggle turns them all off; otherwise it turns them all on. Brightness
and warmth apply one absolute value to every selected bulb; the dashboard shows
the aggregate span and starts each slider at its high end. Colour applies one
packed colour to every bulb. Each bulb is addressed independently,
so a slow bulb does not delay the others; a failure names the bulbs that failed
while the successes stay applied.

## State

The last values read from a bulb: power, brightness, colour temperature, packed
colour and the active colour mode. State is read on launch, on refresh and after
a command fails; it is not cached across runs.

## Unreachable

A bulb that did not answer its hello or command before the timeout. The rail
keeps the bulb and its metadata, the row shows a warning dot, the detail page
shows the failure inline, and `Ctrl+R` retries.

## Restored

A bulb whose pre-sync colour or temperature was written and acknowledged during
stop. The restore report counts it.

## Not restored

A bulb that stopped without a confirmed snapshot, without a restore value, or
with a failed confirmed send. The restore report names it and the other bulbs
still restore.

## Power

The boolean `siid 2 / piid 1` property: a bulb is either on or off.

## Brightness

The `siid 2 / piid 2` property from 1 to 100, shown as a percentage. The
slider is continuous; the keyboard nudges in one-point steps.

## Colour temperature

The `siid 2 / piid 3` property in kelvin, from 2700 K to 6500 K, shown as
Warmth. The slider is continuous; the keyboard nudges in 100 K steps. It is
one of the two light modes.

## Packed colour

The `siid 2 / piid 4` property as one packed 24-bit RGB integer. The picker
edits hue and saturation or the RGB channels and accepts a six-digit hex value.
Every value it can produce is an sRGB colour, so the swatch is exactly what is
sent.

## Mode

The light's active side: White uses colour temperature, Colour uses packed RGB.
The two are mutually exclusive on the bulb. The mode is read from
`siid 2 / piid 6` and is unknown until the first state arrives.

## White

The colour-temperature side of the light. Choosing White applies the bulb's
current temperature, or 4000 K when it has none.

## Colour

The RGB side of the light. Choosing Colour applies the bulb's current colour, or
white when it has none.

## Mixed

The aggregate state when the selected bulbs disagree on power, mode or colour.

## Aggregate

The computed state of every selected bulb: total, reachable, failed, power,
quality, brightness span, temperature span and colour. The dashboard draws it
when All bulbs is selected.

## Default selection

All bulbs on first load when two or more bulbs exist. Choosing a bulb overrides
it until the next launch; adding the second bulb switches to All, adding a later
bulb keeps the current choice.

## Dashboard

The main page: the bulb rail on the left, the selected bulb's power, light mode,
brightness, warmth or colour, and its status on the right.

## Bulb rail

The sidebar list of All bulbs and every configured bulb. A row shows the name,
address, power and a status dot. Double-clicking a row opens Edit.

## Toolbar

The strip above the dashboard. It shows the selected bulb's name and the add,
refresh, sync and settings actions.

## Toast

A transient notification in the bottom-right overlay. Toasts queue, dismiss on
their own after a few seconds, and can be dismissed with a click. Errors and successes use
the same surface with a different tone.

## Screen sync

The mode that drives the configured bulbs from one captured monitor. It is off
until a sync run starts. It lives on the Sync page, not in a separate window.

## Sync run

One start-to-stop period of screen sync. A run is not a miIO session: it covers
the configured bulb set and keeps its own snapshot, reducer state, flash guard
and 1.5 Hz tick.

## Paused

A sync run that keeps capture and state but publishes nothing. The bulbs stay on
the last derived colour and sync brightness. Resume catches up with the current
scene instead of replaying it.

## Sync strip

The banner above the dashboard while a sync run exists or has just stopped. It
carries the state, derived colour, sync brightness and a stale marker.

## Sync page

The page opened with `Ctrl+S`. It owns start/stop, pause, the captured monitor,
the gain and the restore report. It is the only place the sync controls live.

## Captured monitor

The monitor a sync run reads. The monitor list comes from the Wayland outputs.

## Derived colour

The single colour the reducer produces for a frame: the colour of the winning
OKLab cluster — its mean lightness and hue, with the chroma raised toward the
cluster's 75th percentile. It is sent as it is, white, grey and black included.

## Scene brightness

The mean per-pixel drive of the captured content, `max(r, g, b) / 255`, from 0
to 1, published separately from the derived colour.

## Sync brightness

The bulb brightness derived from scene brightness through the 8–100% envelope
and the gain.

## Sync brightness gain

The multiplier applied to the rounded envelope result, from `0.5` to `2.0` in
`0.1` steps. It is `1.0` on a first run and is remembered across launches in the
preferences file. It is adjusted on the Sync page. The gained brightness is
clamped at `100%`; the 8% floor applies to the result, so a gain above `1.0`
raises the floor and a gain below `1.0` does not lower it. The derived colour is
never changed by the gain. The flash guard watches the gained brightness.

## Flash guard

The rule that watches the last three target sync brightness values. A direction
flip whose reversing change is greater than 25 points holds the applied
brightness for two seconds. A trigger disarms the guard; it re-arms only after
three same-direction targets or a peak-to-peak of 25 or less.

## Flashing scene

A sync run whose flash guard is holding brightness. Colour keeps updating; the
Sync page says `Flashing scene` until the hold ends.

## Stale frame

A captured frame older than about five seconds. The run holds the last colour
and stops publishing until a fresh frame arrives.

## Capture lost

A sync run whose capture thread reported an error. Publishing stops but the run
stays alive; stopping restores the bulbs.

## Self-capture warning

The warning on the Sync page when the captured monitor is the only monitor:
jyoti may be capturing its own window. Monitor selection is the only self-mask;
the run is allowed with the warning.

## Restore report

The summary shown after a run stops: how many bulbs restored, and which could
not. A bulb with no snapshot, no restore value or a failed confirmed send is
named.

## Revert

Leaving the colour page with Cancel or `Esc` restores the colour and mode each
target had when the page opened. Done keeps the current colour.

## miIO session

One socket-and-device conversation with a bulb: its own reader, writer, device ID
and message counter. Each bulb has one worker and one session; a sync run drives
the workers it finds.
