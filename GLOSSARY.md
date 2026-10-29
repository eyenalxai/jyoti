# Glossary

## Bulb

A Xiaomi light on the local network. A bulb record has an ID, a display name, an IP address and an optional model. The token is not part of the record; it lives in the keyring. There is no fixed count and no numeric index.

## Bulb ID

The stable identifier jyoti generates when a bulb is added. It is a UUID stored in the config file and used as the keyring account name.

## Keyring

The operating system's Secret Service store, reached through `secret-tool`. It holds one token per bulb under the `jyoti` service. The keyring is the only place a token is stored.

## Config file

`$XDG_CONFIG_HOME/jyoti/bulbs.json`, falling back to `~/.config/jyoti/bulbs.json`. It holds the bulb list's metadata and a format version. It never holds a token.

## Token

The 16-byte device key, written as 32 hex characters. It authenticates and encrypts every miIO packet. It is stored in the keyring, never in the config file, never committed and never logged.

## Device ID

The four-byte identifier a bulb returns in its hello reply. The next request echoes it so the bulb can associate the reply with the request.

## miIO

Xiaomi's UDP protocol on port 54321. A request sends a 32-byte hello, receives the device ID and stamp, then sends an AES-encrypted JSON command. `bulbs.py` is the reference implementation.

## All bulbs

A selection that stands for every configured bulb at once. It is not a bulb: it has no ID, address, token or stored state, so edit and remove do not act on it. The row appears first in the rail while two or more bulbs exist. Its state is the aggregate of the bulbs: power is on, off or mixed; brightness and colour temperature show one value when equal and a range when not; colour shows a swatch when equal and mixed otherwise. Power uses a quorum: if every reachable bulb is on, one toggle turns them all off; otherwise it turns them all on. Brightness and warmth set one absolute value on every bulb, anchored at the aggregate's high end for an increase and its low end for a decrease. Colour applies one packed colour to every bulb. Each bulb is addressed independently, so a slow bulb does not delay the others; a failure names the bulbs that failed while the successes stay applied.

## State

The last values read from a bulb: power, brightness, color temperature, packed color and the active color mode. State is read on launch and on refresh; it is not cached across runs.

## Unreachable

A bulb that did not answer its hello or command before the timeout. The panel keeps the bulb and its metadata, shows the failure inline and offers `r` to retry.

## Power

The boolean `siid 2 / piid 1` property: a bulb is either on or off.

## Brightness

The integer `siid 2 / piid 2` property, from `1` to `100` percent.

## Color temperature

The integer `siid 2 / piid 3` property in kelvin, from `2700` to `6500`.

## Color

The packed 24-bit RGB value in the `siid 2 / piid 4` property: `(r << 16) | (g << 8) | b`. The UI presents it as `#RRGGBB`.

## Color mode

The `siid 2 / piid 6` property that reports which control is active: `1` for color, `2` for color temperature.

## Tokens file

`~/.local/share/xiaomi-bulbs/tokens.env`, written by the reference setup script. jyoti does not read or write it; it exists for `bulbs.py`.

## Screen sync

The mode that drives the configured bulbs from one captured monitor. It is off until a sync run starts.

## Sync run

One start-to-stop period of screen sync. A run is not a miIO session: it covers the configured bulb set and keeps its own snapshot, filter state and 1.5 Hz tick.

## Paused

A sync run that keeps capture, sessions and state but publishes nothing. The bulbs stay on the last derived colour and sync brightness.

## Captured monitor

The monitor a sync run reads.

## Derived colour

The single colour the colour reducer produces for a frame: a chroma-weighted OKLab dominant colour, or the neutral fallback.

## Scene brightness

The mean OKLab lightness of the captured frame, from 0 to 1, published separately from the derived colour.

## Sync brightness

The bulb brightness derived from scene brightness through the 8–80% envelope.

## Blackout hold

Holding the last derived colour and sync brightness while the frame is near-black, so a black scene does not pump the room through neutral.

## Stale frame

A captured frame older than about one second. The run holds the last colour until a fresh frame arrives.

## Neutral fallback

The 4000 K temperature mode used when the scene has too little chroma for a colour.

## miIO session

One socket-and-device conversation with a bulb: its own reader, writer, device ID and message counter. A sync run opens one per bulb; it is not the same thing as the run.
