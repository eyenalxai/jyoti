# jyoti

A native desktop control panel for Xiaomi light bulbs. One Rust process runs a
Rune window and speaks Xiaomi’s miIO protocol to the bulbs over the local
network. There is no daemon, no database, no cloud account and no client/server
split.

Bulbs are added inside the app. Their tokens go to the OS keyring; only
non-secret metadata is written to disk.

## What it does

- Lists any number of bulbs, with live power, brightness, colour temperature
  and RGB colour.
- Adds, edits and removes bulbs without touching a config file by hand.
- Sends power, brightness, warmth and colour changes to the selected bulb, or to
  every bulb when **All bulbs** is selected, over UDP.
- Selects **All bulbs** on first load when two or more bulbs exist.
- Keeps every action on a visible pointer control. A transient toast reports a
  bulb that does not answer.
- Follows the desktop’s light or dark appearance, with a Light, Dark or System
  preference in Settings.
- Uses the Lamplight window: a light stage that carries the selected colour, one
  control deck, quiet underline tabs, warm paper in light and espresso in dark.
- Sets names and page titles in Fraunces, the interface in Inter and addresses,
  hex values and paths in IBM Plex Mono. All three ship with the app.
- Opens at 500 × 680 and can shrink to 480 × 600. The 484-point page stays
  centered as the window grows.
- Starts empty. It does not read the reference script’s `tokens.env`.

## Design

The visual system is **Lamplight**, derived from the product rather than borrowed
from a UI kit. Warm paper in light and espresso in dark carry a brass accent;
the light is the only surface that carries a full colour. Every token lives in
`src/ui/theme.rs`.

- **Light stage** — Control opens on a full-width field filled with the light
  the bulbs are showing. A cream nameplate on the field carries the state, the
  light name and the brightness, so the readout stays legible over any colour.
- **Control deck** — power, brightness, mode and warmth or colour sit in one
  panel of hairline-separated rows. A screen sync run dims the rows it owns and
  keeps power available. The colour popover is modal; the page behind it is inert
  while it is open.
- **Sync stage** — the same surface language shows the captured colour, frame
  age and output brightness; capture and output controls sit beneath it.
- **Type** — Fraunces for names and page titles, Inter for the interface and
  tabular figures, IBM Plex Mono for data. No role sets text below 12 points.
- **Motion** — spring-based throughout, except the appearance switch, which
  lands at once. Section changes push the content below the tab bar horizontally
  in tab order; the tabs stay put. A form or the remove confirmation appears in
  that same content area. The colour editor opens as a popover above the colour
  row and never moves the page. The White/Colour tuning rows swap sideways
  instead of cross-fading. There is no page-load animation, and every action
  stays reachable from a pointer.

## Requirements

- Rust 1.99.1, installed by `rust-toolchain.toml`.
- A Wayland desktop for screen sync. The capture path uses the wlroots
  `wlr-screencopy` protocol, so Hyprland, Sway and River work; GNOME and KDE
  do not yet. X11 is not supported.
- EGL, GBM, DRM and Wayland client development packages for the GPU capture
  path.
- An unlocked Secret Service keyring (`gnome-keyring`, KWallet or another
  Secret Service implementation). `libsecret-tools` is needed only to read
  legacy `secret-tool` tokens once.
- Bulbs on the same LAN, with local control enabled and their tokens to hand.

On Arch the capture packages are `mesa`, `libdrm`, `wayland` and
`wayland-protocols`. On Debian/Ubuntu they are `libegl1-mesa-dev`,
`libgbm-dev`, `libdrm-dev`, `libwayland-dev` and `wayland-protocols`.

The app still controls bulbs without a working capture path; only screen sync
is unavailable.

## Run

```sh
cargo run --release
```

The first screen is empty. Choose **Add bulb** on Control, enter the bulb’s
name, IP address and 32-character token, and save. The token is written to the
keyring; the config file receives the name and address.

For a quick UI check without touching the configured bulbs:

```sh
cargo run -- --demo --scene dashboard --snap dashboard.png
```

`--scene` accepts `dashboard`, `empty`, `form`, `edit`, `color`, `color-control`,
`remove`, `sync`, `sync-live`, `settings`, `toast`, `picker` and `menu`.
`--list-scenes` prints them.

## Storage and security

- Metadata: `$XDG_CONFIG_HOME/jyoti/bulbs.json`, falling back to
  `~/.config/jyoti/bulbs.json`.
- Preferences: `$XDG_CONFIG_HOME/jyoti/prefs.json`, next to the metadata. It
  holds the sync brightness gain, colour intensity and the Light/Dark/System
  appearance preference.
- Tokens: one OS keyring entry per bulb under the `jyoti` service, keyed by the
  bulb’s stable ID. They never enter the config file, a log line or an error
  message. Legacy `secret-tool` entries are read and migrated on first use.
- Both files are written atomically with mode `0600`.
- Overrides: `JYOTI_CONFIG_PATH`, `JYOTI_KEYRING_SERVICE`, `JYOTI_MIIO_PORT`,
  `JYOTI_MIIO_TIMEOUT`.

## Screen sync

Screen sync captures one Wayland monitor, derives its dominant colour and
brightness, and drives every configured bulb. It uses `libwayshot` with the
EGL/DMA-BUF path: the compositor’s frame is imported as an EGLImage, bound to a
GL texture and reduced to a 64×36 grid on the GPU. Only that small grid reaches
the CPU reducer.

- The monitor list is read from the Wayland outputs. A monitor can be chosen
  while stopped or switched during a run.
- A run captures at 1.5 Hz. Pause holds the last colour; resume catches up with
  the current scene.
- The derived colour is the winning OKLab cluster’s mean lightness and hue,
  with its chroma raised toward the cluster’s 75th percentile. It is the input
  to colour intensity.
- **Colour intensity** runs from 0% to 100%. At 0% the derived colour is sent
  unchanged; at 100% its saturation is raised to full while hue and value stay
  put. It is adjusted on Sync and remembered in `prefs.json`.
- Sync brightness follows the frame’s mean drive through an 8–100% envelope and
  the gain. The gain scales only the brightness, from 50% to 400% in 0.1 steps,
  and is remembered in `prefs.json`.
- The flash guard holds brightness for two seconds when the target reverses by
  more than 25 points. Colour keeps updating.
- Power is never written by sync. Colour is sent to every configured bulb;
  brightness only to bulbs known to be on.
- Stop restores each bulb’s pre-sync colour or temperature and the brightness
  sync drove, with confirmed sends bounded to about two seconds. A bulb that
  cannot be restored is named in the toast, and the run returns to idle.
- A bulb that misses sends is marked unreachable, probed with a hello every five
  seconds and rejoins when it answers.

Capture sits behind a `Capture` trait. A ScreenCast portal/PipeWire backend can
cover GNOME and KDE later, and macOS and Windows can add their own backends
without changing the sync reducer or the UI.

## Layout

```
src/
  main.rs              process entry point and Rune AppSpec
  app.rs               app state, poll and lifecycle
  app/
    state.rs           screen, form, colour, toast and sync state
    selection.rs       bulb selection, overlay state, colour editor exit
    color.rs           colour editor state, commit and restore
    actions.rs         bulb CRUD, controls, forms and toasts
    sync.rs            sync run, scene application and restore
  bulb/
    model.rs           bulb records, states, colours and validation
    aggregate.rs       All bulbs power, quality and value spans
    selection.rs       All bulbs or one bulb
    store.rs           bulbs.json and prefs.json with atomic writes
    keyring.rs         OS keyring adapter and legacy migration
    worker.rs          one miIO worker thread per bulb
  miio/
    packet.rs          hello, AES-CBC packet codec and checksum
    properties.rs      siid/piid property parameters and reply errors
    reply.rs           miIO JSON replies
    session.rs         UDP session, hello, exchange and confirmation
    state.rs           state parsing from a reply
  screen/
    capture.rs         ScreenHandle thread and libwayshot GPU capture
    color.rs           sRGB, OKLab, OKLCH, kelvin and HSB conversions
    grid.rs            content rect and prepared frame
    kmeans.rs          deterministic k-means
    reducer.rs         winning-cluster colour and deadband
    brightness.rs      scene brightness and gain envelope
    flash.rs           flash guard
    gain.rs            sync brightness gain
    intensity.rs       sync colour-intensity boost
    monitor.rs         monitor and scene records
    sync.rs            sync status and restore report
  ui/
    theme.rs           Lamplight palette and type scale
    shell.rs           centered toolbar and quiet section tabs
    dashboard.rs       bulb selector and the control deck
    dashboard/
      stage.rs         light stage, nameplate and empty state
    forms.rs           add, edit and remove screens
    color.rs           colour editor popover controls
    sync.rs            capture stage, actions and output controls
    settings.rs        appearance, storage and about
    toast.rs           transient notifications
    widget.rs          icons, shared controls and slider
assets/
  fonts/               bundled OFL families and their licences
docs/
  research/            cited technical notes
  adr/                 current architecture decisions
```

## Commands

| Command | What it does |
| --- | --- |
| `cargo run` | Run the app |
| `cargo run --release` | Run the optimised app |
| `cargo test` | Run the focused unit tests |
| `cargo fmt --check` | Check formatting |
| `cargo clippy --all-targets --no-deps -- -D warnings` | Lint |
| `cargo check` | Type-check |
| `cargo build --profile perf` | Build the benchmark profile |

## License

MIT.
