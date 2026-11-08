# jyoti

A native desktop control panel for Xiaomi light bulbs. One Rust process runs a
Rune window and speaks Xiaomi's miIO protocol to the bulbs over the local
network. There is no daemon, no database, no cloud account and no client/server
split.

Bulbs are added inside the app. Their tokens go to the OS keyring; only
non-secret metadata is written to disk.

## What it does

- Lists any number of bulbs, with live power, brightness, colour temperature and
  RGB colour.
- Adds, edits and removes bulbs without touching a config file by hand.
- Sends power, brightness, warmth and colour changes to the selected bulb, or to
  every bulb when **All bulbs** is selected, over UDP.
- Selects **All bulbs** on first load when two or more bulbs exist.
- Keeps every action on a visible pointer control. A transient toast reports a
  bulb that does not answer.
- Follows the desktop's light or dark appearance.
- Uses a dense ReUI-derived shell: grouped sidebar, KPI tiles, flat panels and
  a compact bulb table, with every action on a pointer control.
- Starts empty. It does not read the reference script's `tokens.env`.

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

The first screen is empty. Choose **Add bulb**, enter the bulb's name, IP
address and 32-character token, and save. The token is written to the keyring;
the config file receives the name and address.

For a quick UI check without touching the configured bulbs:

```sh
cargo run -- --demo --scene dashboard --snap dashboard.png
```

`--scene` accepts `dashboard`, `empty`, `form`, `edit`, `color`, `remove`,
`sync`, `sync-live`, `settings`, `help` and `toast`. `--list-scenes` prints them.

## Storage and security

- Metadata: `$XDG_CONFIG_HOME/jyoti/bulbs.json`, falling back to
  `~/.config/jyoti/bulbs.json`.
- Preferences: `$XDG_CONFIG_HOME/jyoti/prefs.json`, next to the metadata. It
  holds the sync brightness gain.
- Tokens: one OS keyring entry per bulb under the `jyoti` service, keyed by the
  bulb's stable ID. They never enter the config file, a log line or an error
  message. Legacy `secret-tool` entries are read and migrated on first use.
- Both files are written atomically with mode `0600`.
- Overrides: `JYOTI_CONFIG_PATH`, `JYOTI_KEYRING_SERVICE`, `JYOTI_MIIO_PORT`,
  `JYOTI_MIIO_TIMEOUT`.

## Screen sync

Screen sync captures one Wayland monitor, derives its dominant colour and
brightness, and drives every configured bulb. It uses `libwayshot` with the
EGL/DMA-BUF path: the compositor's frame is imported as an EGLImage, bound to a
GL texture and reduced to a 64×36 grid on the GPU. Only that small grid reaches
the CPU reducer.

- The monitor list is read from the Wayland outputs. A monitor can be chosen
  while stopped or switched during a run.
- A run captures at 1.5 Hz. Pause holds the last colour; resume catches up with
  the current scene.
- The derived colour is the winning OKLab cluster's mean lightness and hue,
  with its chroma raised toward the cluster's 75th percentile. It is sent as it
  is, white, grey and black included.
- Sync brightness follows the frame's mean drive through an 8–100% envelope and
  the gain. The gain scales only the brightness, from 50% to 200% in 0.1 steps,
  and is remembered in `prefs.json`.
- The flash guard holds brightness for two seconds when the target reverses by
  more than 25 points. Colour keeps updating.
- Power is never written by sync. Colour is sent to every configured bulb;
  brightness only to bulbs known to be on.
- Stop restores each bulb's pre-sync colour or temperature and the brightness
  sync drove, with confirmed sends bounded to about two seconds. A bulb that
  cannot be restored is named in the report.
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
    selection.rs       sidebar and table selection, colour-page exit
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
    monitor.rs         monitor and scene records
    sync.rs            sync status and restore report
  ui/
    shell.rs           sidebar, toolbar and split view
    dashboard.rs       KPI row, bulb table, power and light controls
    forms.rs           add, edit and remove pages
    color.rs           hue/saturation and RGB colour picker
    sync.rs            screen sync page
    settings.rs        preferences, storage and about
    help.rs            help page
    toast.rs           transient notifications
    widget.rs          icons and shared controls
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
