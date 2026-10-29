# jyoti

A terminal control panel for Xiaomi light bulbs. One Bun process runs an OpenTUI (Solid) interface and speaks Xiaomi's miIO protocol to the bulbs over the local network. There is no daemon and no client/server split.

Bulbs are added inside the app. Their tokens go to the OS keyring; only non-secret metadata is written to disk.

## What it does

- Lists any number of bulbs, with live power, brightness, color temperature and RGB color.
- Adds, edits and removes bulbs without touching a config file by hand.
- Sends power, brightness, warmth and color changes to the selected bulb, or to every bulb when All bulbs is selected, over UDP.
- Selects All bulbs on first load when two or more bulbs exist, so the common job starts in one place.
- Keeps every action on the keyboard, with a transient toast when a bulb does not answer.
- Uses the terminal's default foreground, background and ANSI palette, so it follows the active terminal theme.
- Starts empty. It does not read `~/.local/share/xiaomi-bulbs/tokens.env`; that file remains the reference script's input.

## Requirements

- Bun 1.4 or newer
- `libsecret-tools` and an unlocked Secret Service keyring (`secret-tool` on `PATH`)
- Bulbs on the same LAN, with local control enabled and their tokens to hand
- For screen sync only: `cc`, `pkg-config`, `wayland-scanner`, and the development packages for `wayland-client`, `egl`, `gbm` and `libdrm`

On Debian/Ubuntu: `sudo apt install libsecret-tools`. On Arch: `sudo pacman -S libsecret`.

Screen sync builds a small C library against the local Wayland stack. The rest of jyoti runs without it; a missing library only disables screen sync.

## Run

```sh
bun install
bun start
```

The first screen is empty. Press `a` and enter the bulb's name, IP address and 32-character token. The token is written to the keyring; the config file receives the name and address.

To use screen sync, build the native capture library first:

```sh
bun run build:native
```

The build writes `native/build/libjyoti-capture.so` and the generated Wayland protocol bindings. Both are build artefacts and stay out of git.

## Keys

| Key               | Action                                                                           |
| ----------------- | -------------------------------------------------------------------------------- |
| `↑` `↓` / `j` `k` | Select a bulb, or move the focused control                                       |
| `A`               | Select all bulbs (two or more)                                                   |
| `tab`             | Switch between the bulb list and controls, or between panes in a narrow terminal |
| `←` `→` / `h` `l` | Adjust the focused control                                                       |
| `space` / `p`     | Toggle power                                                                     |
| `enter`           | Open controls, or act on the focused control                                     |
| `w`               | Switch to white light                                                            |
| `c`               | Switch to colour and open the picker                                             |
| `a`               | Add a bulb                                                                       |
| `e`               | Edit the selected bulb                                                           |
| `d`               | Remove the selected bulb                                                         |
| `r`               | Refresh the list and all states                                                  |
| `s`               | Start or stop screen sync                                                        |
| `x`               | Pause or resume screen sync                                                      |
| `+` `-`           | Adjust the sync brightness gain                                                  |
| `m`               | Choose the monitor screen sync captures                                          |
| `v`               | Capture preview (development preview)                                            |
| `?`               | Show the key map                                                                 |
| `q` / `ctrl+c`    | Quit                                                                             |

Brightness moves in 5% steps and warmth in 100 K steps. Hue moves in 1° steps and saturation in 1% steps. Changes update immediately and are sent after a short idle; a failed command restores the bulb's real state and reports the failure in a toast.

Notifications and errors appear as transient toasts in the top-right corner. They queue, dismiss on their own, and can be dismissed with a click.

Each bulb has two modes: **White** (colour temperature) and **Colour** (packed RGB). They are mutually exclusive on the bulb, so the panel shows the mode first and then that mode's own settings: brightness and warmth for White, and the colour for Colour. `←` `→` on the mode row switches it, and `w` and `c` do the same from anywhere. The colour picker edits hue and saturation; every value it can produce is an sRGB colour, so the swatch is exactly what is sent. `tab` switches between hue/saturation and RGB, the hex field accepts an exact colour, `↑` `↓` move between fields, and `←` `→` adjust the focused slider. A black colour opens on RGB, since hue and saturation cannot leave black. Each change is sent after a short idle, so the bulb follows the picker. `enter` keeps the colour and closes; `esc` reverts to the colour and mode each target had when the picker opened.

The shell is responsive. The bulb list and the controls sit side by side from 72 columns; below that, `tab` switches one full-width pane at a time. The key bar wraps onto as many centred rows as it needs and is never truncated, and the panel asks for at least 40 columns and 14 rows.

With two or more bulbs, **All bulbs** is selected on first load and `A` returns to it. The panel shows the aggregate power, the mode and that mode's settings. When the bulbs disagree on the mode the switch shows `mixed`, no mode's settings are drawn, and choosing White or Colour applies it to every bulb. `space` turns every bulb off when all of them are on, or on otherwise. `←` `→` set the same brightness or warmth on every bulb while White is shown, starting from the aggregate — right from the brightest, left from the dimmest — and `c` applies one colour to all of them as you adjust; `esc` reverts each bulb's own previous colour. Each bulb is addressed independently, so one slow bulb does not hold up the rest; a partial failure names the bulbs that failed and keeps the successes. Edit and remove stay per bulb.

## Screen sync

Screen sync is being built in slices. `s` starts a sync run that captures one monitor, derives its colour at 1.5 Hz and drives every configured bulb; `s` again stops the run. The bulb set is snapshotted at start, so add, edit and remove are blocked until sync stops. `x` pauses and resumes publishing: pause holds the last derived colour, and resume catches up with the current scene instead of replaying it.

The default captured monitor is the only monitor, otherwise the first monitor that is not focused, otherwise the first enumerated monitor. `m` lists the monitors with their name, resolution, refresh rate and focus marker. A choice while stopped applies to the next run only; a choice during a run switches immediately; the choice is never persisted. A single monitor, or unknown focus, is captured with a persistent `⚠ jyoti` warning in the strip.

The two-line sync strip above the footer shows the state, captured monitor, derived colour, sync brightness, frame age and reachable/total bulbs. It reports starting, paused, capture stale, capture lost, switching monitor, flashing scene and stopping, and its second line carries the run keys. Per-bulb dots in the rail show syncing, unreachable, restored and not restored. A stale frame holds the last colour and brightness; after five seconds publishing stops but the run stays alive.

A scene writes the packed RGB value of the winning cluster's own colour — its mean lightness and hue, with the chroma raised toward the cluster's 75th percentile — so a solid colour round-trips as itself, white, grey and black included. Sync never writes colour temperature; temperature stays in the manual controls and the pre-sync restore. Sync brightness follows the scene brightness, the frame's mean drive, through an 8–100% envelope; `+` and `-` scale that result with a gain from 50% to 200% (100% by default, 10% steps, `=` also raises), shown as `68% ×1.5` in the strip while it is not 100%. While paused it shows `45% · gain ×1.5`, since the brightness stays at the last written value until resume. The gain applies on the next tick, survives stop and start within one process and resets to 100% on launch. Sync brightness is written on the tick it changes, the bulb's own fade smooths it as it does for colour, and a bulb that turns on later picks up the current target on the next tick. It is written only to bulbs known to be on: an off bulb still receives colour so it is current when switched on, but never a brightness write, and a bulb with unknown power receives colour only. Power is never written. When three brightness targets flip direction and the reversing change is over 25 points, the flash guard holds the current brightness for two seconds, shows `flashing scene` and keeps updating colour at the normal tick. It re-arms after three same-direction targets or a peak-to-peak of 25 or less. While a run is active, the colour, brightness and warmth controls are disabled and say that sync owns them; power, refresh and navigation keep working. The TUI feeds the run's known power from its power actions and refreshes.

Stopping restores each bulb's pre-sync colour or temperature and, when the run drove it, brightness, with confirmed sends bounded to about two seconds: a bulb with no confirmed snapshot, no restore value or a failed restore is named as not restored while the others restore. `q` and `ctrl+c` restore before exiting. A bulb that misses three consecutive sends is marked unreachable, probed with a hello every five seconds and rejoins the run automatically when it answers.

The capture library is built from source and loaded lazily with `bun:ffi`. A missing library is reported as a typed error at capture start and leaves every other jyoti feature working.

`v` opens a development preview. It lists the monitors with their resolution, refresh rate and the focused one, captures the selected monitor through the native library, and shows the derived colour, the scene brightness and the frame age. `enter` starts or stops the capture, `↑` `↓` choose a monitor, and `esc` closes the preview. The preview does not write to any bulb.

## Storage and security

- Metadata: `$XDG_CONFIG_HOME/jyoti/bulbs.json`, falling back to `~/.config/jyoti/bulbs.json`. The file is written atomically with mode `0600`.
- Tokens: Secret Service entries under the `jyoti` service, one per bulb. They never enter the config file, a log line or an error message.
- Overrides: `JYOTI_CONFIG_PATH`, `JYOTI_KEYRING_SERVICE`, `JYOTI_MIIO_PORT`, `JYOTI_MIIO_TIMEOUT`.

## Commands

| Command                | What it does                     |
| ---------------------- | -------------------------------- |
| `bun install`          | Install dependencies             |
| `bun start`            | Run the TUI                      |
| `bun run dev`          | Run the TUI in watch mode        |
| `bun run build:native` | Build the screen capture library |
| `bun run check`        | Format check, lint and typecheck |
| `bun run format`       | Format with oxfmt                |
| `bun run lint`         | Lint with oxlint                 |
| `bun run tsc`          | Typecheck                        |

## Layout

```
src/
  main.tsx                 process entry point
  components/
    app.tsx                screen state and actions
    bulb/                  dashboard, rail, detail, forms, keyboard
    screen/                sync strip, monitor picker, capture preview
    ui/                    modal, meter, toast, key hint
  lib/
    bulbs/                 schemas, config file store, repository
    keyring/               Secret Service adapter
    miio/                  packet codec, UDP transport, bulb client
    screen/                capture, colour reducer and sync run
    config.ts              runtime configuration
    runtime.ts             Effect runtime boundary
    theme.ts               palette
native/
  capture.c                C11 wlr-screencopy capture library
  protocol/                vendored Wayland protocol XML
  build.ts                 bun run build:native
```

`bulbs.py` remains the reference implementation for the wire format; jyoti is a TypeScript port and does not shell out to it.

## Documentation

- `AGENTS.md` — working agreements for this repository.
- `GLOSSARY.md` — the language used for bulbs, tokens and miIO.
- `docs/adr/` — the direct-connection and keyring decisions.
- `docs/agents/issue-tracker.md` — Linear team, labels and publishing conventions.

## License

MIT
