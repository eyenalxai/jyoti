# Rust libraries for the jyoti rewrite

Date: 2026-11-08
Status: answered

## Question

Which parts of the current TypeScript and C implementation can be replaced by
maintained Rust crates, and which library should capture a Wayland monitor on
the GPU?

## Answer

Use `libwayshot` 0.10.0 with its `egl`, `dmabuf` and `screencast` features. It
wraps `zwlr_screencopy_v1` and `ext-image-copy-capture-v1`, imports the
compositor's DMA-BUF into an EGLImage and binds it to a GL texture. The
full-frame downsample stays on the GPU; only the reduced grid reaches the CPU.
It replaces the C capture library, its vendored Wayland protocol XML and the
capture FFI.

The rest of the plumbing maps to maintained crates:

| Area | Crate | Why |
| --- | --- | --- |
| OS keyring | `keyring` 4.2 | One API for Secret Service, macOS Keychain and Windows Credential Manager. |
| Colour | `palette` 0.7 | sRGB, OKLab and OKLCH conversions in one mature crate. |
| miIO crypto | `aes` 0.9, `cbc` 0.2, `md-5` 0.11 | RustCrypto implements the packet's AES-128-CBC and MD5 pieces. |
| JSON and config | `serde` 1, `serde_json` 1 | Typed bulb records, preferences and miIO replies. |
| XDG paths | `directories` 6 | Config paths on Linux now, the right paths on macOS and Windows later. |
| Atomic writes | `tempfile` 3 | Write a sibling temp file and rename it over the config. |
| Bulb IDs | `uuid` 1 | Stable UUIDs for the config and keyring account name. |
| Errors | `thiserror` 2 | Typed errors per subsystem without hand-written `Display` code. |

No maintained high-level miIO crate exists. The packet codec is small and stays
app code on top of RustCrypto.

Roughly 60% of the current 13.9k lines is framework, capture, protocol and store
plumbing that these libraries replace. The other 40% is product logic: the bulb
aggregate, the sync reducer, restore, and the app's state and views.

## Evidence

### Screen capture

- `libwayshot` 0.10.0, BSD-2-Clause, repository `waycrate/wayshot`.
  crates.io reports 383,480 total downloads, 127,099 recent downloads and a
  latest release on 2026-11-08; 0.9.0 shipped on 2026-08-04. Source:
  <https://crates.io/api/v1/crates/libwayshot>.
- The crate's feature table says `egl` enables the "EGL/OpenGL GPU capture
  backend (DMA-BUF → EGLImage → GL texture)" and `screencast` enables `dmabuf`.
  Source: <https://docs.rs/libwayshot/0.10.0/libwayshot/> (README, "EGL capture
  backend").
- The public API includes `capture_target_frame_eglimage`,
  `bind_target_frame_to_gl_texture`, `create_screencast_with_egl` and
  `EGLImageGuard`. Source:
  <https://docs.rs/libwayshot/0.10.0/libwayshot/struct.WayshotConnection.html>.
- The implementation handles `zwlr_screencopy_v1` and
  `ext-image-copy-capture-v1`; monitors come from `OutputInfo` and
  `get_all_outputs`. Source:
  <https://docs.rs/libwayshot/0.10.0/src/libwayshot/lib.rs.html>.
- Caveat: the screencopy protocols are wlroots protocols. Hyprland, Sway and
  River expose them; GNOME and KDE do not. Those desktops need the ScreenCast
  portal and PipeWire. Source: the crate README's first paragraph and the
  protocol names above.
- `scap` 0.1.0-beta.1 reports 62,374 downloads and a last release on
  2025-08-04. It is a beta with more than a year between releases, so it is not
  the primary choice. Source: <https://crates.io/api/v1/crates/scap>.
- `xcap` 0.9.8 reports 2,804,487 downloads and a last release on 2026-08-01. It
  is active, but its API returns images and its Wayland path goes through a
  portal/dbus screenshot; it does not expose a zero-copy GPU path. Source:
  <https://crates.io/api/v1/crates/xcap> and
  <https://docs.rs/xcap/0.9.8/xcap/>.
- `pipewire` 0.10.1 is the official Rust binding from the PipeWire project.
  crates.io reports 3,388,549 downloads and a last release on 2026-08-19. It is
  the right backend for a portal-based path on GNOME and KDE, but a capture
  consumer is more code than `libwayshot`. Source:
  <https://crates.io/api/v1/crates/pipewire> and
  <https://gitlab.freedesktop.org/pipewire/pipewire-rs>.
- `ashpd` 0.14.0 wraps the XDG desktop portals. crates.io reports 20,177,557
  downloads and a last release on 2026-11-05. It is the portal half of that
  future backend. Source: <https://crates.io/api/v1/crates/ashpd>.

### Keyring

- `keyring` 4.2.0, MIT OR Apache-2.0. crates.io reports 38,050,891 downloads,
  18,241,764 recent downloads and a last release on 2026-08-29. Source:
  <https://crates.io/api/v1/crates/keyring>.
- The README states that the default `v1` feature gives "the ability to set,
  get, and delete both plain-text and binary secrets in the native secure
  stores on Mac, Windows, and *nix operating systems". Source:
  <https://docs.rs/keyring/4.2.0/keyring/> (README).
- On Linux the default store is `zbus-secret-service-keyring-store`, the Secret
  Service API. Source: the `keyring` 4.2.0 feature list on
  <https://crates.io/api/v1/crates/keyring>.

### Colour

- `palette` 0.7.7, MIT OR Apache-2.0. crates.io reports 38,344,250 downloads,
  24,167,346 recent downloads and a last release on 2026-08-02. Source:
  <https://crates.io/api/v1/crates/palette>.
- It exports `Oklab` and `Oklch`, which cover the reducer's colour space.
  Source: <https://docs.rs/palette/0.7.7/palette/> and
  <https://docs.rs/palette/0.7.7/palette/struct.Oklch.html>.

### miIO crypto and wire data

- `aes` 0.9.4, RustCrypto, 399,100,250 downloads, last release 2026-11-02.
  <https://crates.io/api/v1/crates/aes>
- `cbc` 0.2.1, RustCrypto, 137,084,764 downloads, last release 2026-05-20.
  <https://crates.io/api/v1/crates/cbc>
- `md-5` 0.11.0, RustCrypto, 448,998,836 downloads, last release 2026-03-27.
  <https://crates.io/api/v1/crates/md-5>
- crates.io search for `miio` returns `rust-py-miio` 0.1.3 (2025-02-11, 2,834
  downloads) and `miio-proto` 0.1.5 (2022-02-18, 7,843 downloads). Neither is a
  maintained foundation for this app. Source:
  <https://crates.io/api/v1/crates?q=miio>.

### JSON, paths, writes, IDs, errors, logs

- `serde` 1.0.229, 1,667,489,961 downloads, last release 2026-07-18.
  <https://crates.io/api/v1/crates/serde>
- `serde_json` 1.0.154, 1,573,283,686 downloads, last release 2026-10-11.
  <https://crates.io/api/v1/crates/serde_json>
- `directories` 6.0.0, 81,130,993 downloads, last release 2025-01-12.
  <https://crates.io/api/v1/crates/directories>
- `tempfile` 3.27.0, 972,461,916 downloads, last release 2026-03-11.
  <https://crates.io/api/v1/crates/tempfile>
- `uuid` 1.29.1, 966,228,544 downloads, last release 2026-11-06.
  <https://crates.io/api/v1/crates/uuid>
- `thiserror` 2.0.21, 1,801,661,096 downloads, last release 2026-09-23.
  <https://crates.io/api/v1/crates/thiserror>

## Mapping

The line counts are from the current tree; the remaining-work figures are
planning estimates, not measurements.

| Area | Current lines | Replacement | Remaining app code |
| --- | --- | --- | --- |
| UI components and layout | ~5,600 | Rune, `rune-kit`, `rune-motion` | ~1,500 |
| Wayland capture | ~2,700 C + ~300 TS | `libwayshot` (`egl`, `dmabuf`) | ~400 (monitor choice, GL downsample) |
| miIO packet and transport | ~1,000 | `aes`, `cbc`, `md-5`, `serde_json` | ~350 |
| Config, keyring, preferences | ~340 | `keyring`, `serde_json`, `directories`, `tempfile`, `uuid` | ~150 |
| Colour conversion | ~500 | `palette` | ~300 (reducer and k-means) |
| Sync run and restore | ~2,100 | none; product logic | ~1,800 |
| App state and formatting | ~500 | `thiserror` | ~400 |
| Total | ~13,900 | | ~4,900 |

## Conflicts

`libwayshot` is the smaller path and it uses the GPU, but it only covers
wlroots compositors. `pipewire` plus `ashpd` covers every Wayland desktop that
implements the ScreenCast portal, at the cost of a consent dialog and a larger
consumer. The app should start with a `Capture` trait and `libwayshot` as the
first backend. A portal backend can be added without changing the sync reducer
or the UI.

`scap` advertises cross-platform capture and would cover macOS and Windows
later, but it is a beta with a year-old release and it returns CPU frames.
`libwayshot` plus a `Capture` trait is the safer Linux-first choice.

## Not found

- A maintained, high-level miIO crate: crates.io has only the two small
  packages above. The packet codec stays app code.
- A maintained pure-Rust wlroots screencopy crate with a GPU path other than
  `libwayshot`: `smithay-client-toolkit` 0.21.1 and `wayland-protocols-wlr`
  0.3.12 provide the protocol pieces, but the EGL/GBM assembly would be app
  code. Source: `smithay-client-toolkit` 0.21.1,
  <https://crates.io/api/v1/crates/smithay-client-toolkit>; and
  `wayland-protocols-wlr` 0.3.12,
  <https://crates.io/api/v1/crates/wayland-protocols-wlr>.

## Recommendation

Adopt the library set in the first table. Use `libwayshot` with `egl` and
`dmabuf` as the Linux capture backend, keep full-frame work on the GPU, and send
only the reduced grid to the CPU reducer. Keep capture behind a trait so a
portal/PipeWire backend can cover GNOME and KDE later, and so macOS and Windows
can add their own backends without touching the sync model.

## References

- `libwayshot` API and EGL backend:
  <https://docs.rs/libwayshot/0.10.0/libwayshot/>
- `libwayshot` source:
  <https://docs.rs/libwayshot/0.10.0/src/libwayshot/lib.rs.html>
- `libwayshot` releases and downloads:
  <https://crates.io/api/v1/crates/libwayshot>
- `keyring` README and stores: <https://docs.rs/keyring/4.2.0/keyring/>
- `palette` API: <https://docs.rs/palette/0.7.7/palette/>
- PipeWire Rust bindings: <https://gitlab.freedesktop.org/pipewire/pipewire-rs>
- XDG portals wrapper: <https://crates.io/api/v1/crates/ashpd>
