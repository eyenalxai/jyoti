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
