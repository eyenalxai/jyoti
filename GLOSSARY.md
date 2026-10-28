# Glossary

## Bulb

A Xiaomi light on the local network. Each bulb has an index (`1`–`3`), a display name, an IP address and a token. The defaults come from `tokens.env`.

## Target

How a command selects bulbs: `all`, a bulb index, a name, or a partial name that matches exactly one bulb.

## Token

The 16-byte device key, written as 32 hex characters in `tokens.env`. It authenticates and encrypts every miIO packet. A token never leaves the machine and is never committed.

## Device ID

The four-byte identifier a bulb returns in its hello reply. The next request echoes it so the bulb can associate the reply with the request.

## miIO

Xiaomi's UDP protocol on port 54321. A request sends a 32-byte hello, receives the device ID and stamp, then sends an AES-encrypted JSON command. `bulbs.py` is the reference implementation.

## Service (`siid`)

A group of related properties inside the miIO schema. The light service used here is `siid = 2`.

## Property (`piid`)

A single value inside a service. The light service exposes power (`1`), brightness (`2`), color temperature (`3`), color (`4`) and the current color mode (`6`).

## Power

The boolean `siid 2 / piid 1` property: a bulb is either on or off.

## Brightness

The integer `siid 2 / piid 2` property, from `1` to `100` percent.

## Color temperature

The integer `siid 2 / piid 3` property in kelvin, from `2700` to `6500`.

## Color

The packed 24-bit RGB value in the `siid 2 / piid 4` property: `(r << 16) | (g << 8) | b`.

## Color mode

The `siid 2 / piid 6` property that reports which control is active: `1` for color, `2` for color temperature.

## Tokens file

`~/.local/share/xiaomi-bulbs/tokens.env`, written by the existing setup script. It holds `BULB_<n>_IP`, `BULB_<n>_TOKEN`, `BULB_<n>_NAME` and `BULB_<n>_MODEL` entries.
