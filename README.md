# jyoti

A terminal control panel for Xiaomi light bulbs. One Bun process runs an OpenTUI (Solid) interface and speaks Xiaomi's miIO protocol to the bulbs over the local network. There is no daemon and no client/server split.

Bulbs are added inside the app. Their tokens go to the OS keyring; only non-secret metadata is written to disk.

## What it does

- Lists any number of bulbs, with live power, brightness, color temperature and RGB color.
- Adds, edits and removes bulbs without touching a config file by hand.
- Sends power, brightness, warmth and color changes to the selected bulb, or to every bulb when All bulbs is selected, over UDP.
- Keeps every action on the keyboard, with an inline error when a bulb does not answer.
- Uses the terminal's default foreground, background and ANSI palette, so it follows the active terminal theme.
- Starts empty. It does not read `~/.local/share/xiaomi-bulbs/tokens.env`; that file remains the reference script's input.

## Requirements

- Bun 1.4 or newer
- `libsecret-tools` and an unlocked Secret Service keyring (`secret-tool` on `PATH`)
- Bulbs on the same LAN, with local control enabled and their tokens to hand

On Debian/Ubuntu: `sudo apt install libsecret-tools`. On Arch: `sudo pacman -S libsecret`.

## Run

```sh
bun install
bun start
```

The first screen is empty. Press `a` and enter the bulb's name, IP address and 32-character token. The token is written to the keyring; the config file receives the name and address.

## Keys

| Key               | Action                                       |
| ----------------- | -------------------------------------------- |
| `↑` `↓` / `j` `k` | Select a bulb, or move the focused control   |
| `A`               | Select all bulbs (two or more)               |
| `tab`             | Switch between the bulb list and controls    |
| `←` `→` / `h` `l` | Adjust the focused control                   |
| `space` / `p`     | Toggle power                                 |
| `enter`           | Open controls, or act on the focused control |
| `c`               | Open the colour picker                       |
| `a`               | Add a bulb                                   |
| `e`               | Edit the selected bulb                       |
| `d`               | Remove the selected bulb                     |
| `r`               | Refresh the list and all states              |
| `?`               | Show the key map                             |
| `q` / `ctrl+c`    | Quit                                         |

Brightness moves in 5% steps, warmth in 100 K steps. Changes update immediately and are sent after a short idle; a failed command restores the bulb's real state and explains what went wrong.

The colour picker starts in OKLCH mode with L, C and H sliders. `tab` switches to RGB sliders; the hex field stays editable in either mode. The swatch and hex readout follow the sliders, `↑` `↓` move between controls, `←` `→` adjust the focused slider, and `enter` applies. Unsupported chroma is reduced to fit sRGB and marked as clipped.

With two or more bulbs, `A` selects **All bulbs**: the panel shows the aggregate power, brightness, warmth and colour. `space` turns every bulb off when all of them are on, or on otherwise. `←` `→` set the same brightness or warmth on every bulb, starting from the aggregate — right from the brightest, left from the dimmest — and `c` applies one colour to all of them. Each bulb is addressed independently, so one slow bulb does not hold up the rest; a partial failure names the bulbs that failed and keeps the successes. Edit and remove stay per bulb.

## Storage and security

- Metadata: `$XDG_CONFIG_HOME/jyoti/bulbs.json`, falling back to `~/.config/jyoti/bulbs.json`. The file is written atomically with mode `0600`.
- Tokens: Secret Service entries under the `jyoti` service, one per bulb. They never enter the config file, a log line or an error message.
- Overrides: `JYOTI_CONFIG_PATH`, `JYOTI_KEYRING_SERVICE`, `JYOTI_MIIO_PORT`, `JYOTI_MIIO_TIMEOUT`.

## Commands

| Command          | What it does                     |
| ---------------- | -------------------------------- |
| `bun install`    | Install dependencies             |
| `bun start`      | Run the TUI                      |
| `bun run dev`    | Run the TUI in watch mode        |
| `bun run check`  | Format check, lint and typecheck |
| `bun run format` | Format with oxfmt                |
| `bun run lint`   | Lint with oxlint                 |
| `bun run tsc`    | Typecheck                        |

## Layout

```
src/
  main.tsx                 process entry point
  components/
    app.tsx                screen state and actions
    bulb/                  dashboard, rail, detail, forms, keyboard
    ui/                    modal, meter, status, key hint
  lib/
    bulbs/                 schemas, config file store, repository
    keyring/               Secret Service adapter
    miio/                  packet codec, UDP transport, bulb client
    config.ts              runtime configuration
    runtime.ts             Effect runtime boundary
    theme.ts               palette
```

`bulbs.py` remains the reference implementation for the wire format; jyoti is a TypeScript port and does not shell out to it.

## Documentation

- `AGENTS.md` — working agreements for this repository.
- `GLOSSARY.md` — the language used for bulbs, tokens and miIO.
- `docs/adr/` — the direct-connection and keyring decisions.
- `docs/agents/issue-tracker.md` — Linear team, labels and publishing conventions.

## License

MIT
