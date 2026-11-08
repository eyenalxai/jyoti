# jyoti

A native desktop control panel for Xiaomi light bulbs. One Rust process runs a
Rune window and speaks Xiaomi's miIO protocol over the local network. There is
no daemon, no database and no HTTP API. `README.md` documents the product,
`GLOSSARY.md` the domain vocabulary.

## Workspace

One Cargo package at the repository root:

- `src/main.rs` — the process entry point and the Rune `AppSpec`.
- `src/app.rs` and `src/app/` — app state, poll, keys, actions and the sync run.
- `src/bulb/` — bulb records, aggregate, selection, config store, keyring and
  the per-bulb miIO worker.
- `src/miio/` — miIO packets, properties, replies, session and state parsing.
- `src/screen/` — libwayshot capture, colour conversion, the reducer, sync
  brightness and the flash guard.
- `src/ui/` — the Rune shell and pages.
- `docs/research/` — cited technical notes. `docs/adr/` — current decisions.

The Rune dependency is pinned to a `dev` rev in `Cargo.toml`. Do not fork it.
`rust-toolchain.toml` pins the compiler.

## Commands

- `cargo run` — run the app.
- `cargo test` — run the focused unit tests.
- `cargo fmt --check` — check formatting.
- `cargo clippy --all-targets --no-deps -- -D warnings` — lint.
- `cargo check` — type-check.
- `cargo run -- --demo --scene dashboard --snap dashboard.png` — render a
  headless snapshot without touching configured bulbs.
- `cargo run -- --scene sync-live --capture-once` — capture one real monitor
  and print its derived colour. It does not write to a bulb.
- `cargo build --profile perf` — benchmark profile.

Run `cargo fmt`, `cargo clippy --all-targets --no-deps`, `cargo test` and
`cargo check` before committing.

## Conventions

- Rust 2024, one binary crate. No `as` casts that silence the type checker, no
  `@ts-ignore` equivalents, no non-null assertions, no lint suppressions.
  Return a typed error instead of `unwrap` or `expect` in production paths.
- Comments only for a hard "why this way?"; code says what it does.
- Rune's single `App` is the view model. Group feature state in its own module
  (`app/sync`, `app/state`, a UI page) and keep handlers with the state they
  change; do not scatter one feature's fields across unrelated modules.
- Bulb tokens live in the OS keyring (`keyring`), never in config, logs or error
  messages. Metadata lives in `$XDG_CONFIG_HOME/jyoti/bulbs.json`; preferences
  in `prefs.json`. Both are written atomically with mode `0600`.
- Wire values follow the miIO schema: `siid`/`piid` properties, integer
  brightness, kelvin temperature and packed RGB. The reference implementation is
  `bulbs.py`; do not copy Python into this repository.
- Screen capture uses `libwayshot` with its EGL/DMA-BUF path. Full-frame work
  stays on the GPU; only the reduced grid reaches the CPU. Keep capture behind
  the `Capture` boundary so another Wayland backend can land later.
- Add a dependency only when the task needs it. Prefer a maintained crate over
  hand-written plumbing.
- Add focused unit tests for pure logic: the packet codec, aggregate, reducer,
  brightness envelope and flash guard. Do not add an integration suite.

## Rune and Rust

The app uses Rune's primitives (`rune`, `rune-kit`, `rune-motion`). Use a kit
component when it fits the screen. A small app-specific wrapper around kit
tokens is fine when the kit's shape does not match; do not reimplement behavior
the kit already provides. Keep Rust files under 800 non-test
lines; 1,500 is a hard cap. Move a growing area into its own module before it
reaches the cap. Shape folders by capability: when `x.rs` sits beside two or
more `x_*.rs`, move them into `x/`. In a module root (`main.rs`, `app.rs`),
keep `mod` lists one item per line, sorted.

### Performance

- Start slow work (network, keyring, bulb discovery, screen capture) after the
  first frame. Land state that arrives just after launch with `tx.instant()`, so
  the first second stays still.
- `poll` returns true only when visible state changed, and a loop stops asking
  for frames when its work ends. An idle window draws nothing.
- Read sizes with `Cx::layout_size` or `Cx::layout_with`, not whole rects, so a
  scroll or resize does not rebuild the view every frame. A handler that only
  moves a live value calls `Tx::live_only()`.
- Build only mounted rows: `virtual_rows`, with row content built inside its
  closure; `memo` for rows that rarely change. Virtualise a list when it can
  outgrow the viewport; the configured-bulb table uses the kit table.
- Bound every cache in entries and in bytes, counting decoded pixels, not
  encoded size. Screen-sync frames, thumbnails and colour reductions obey the
  same budget.
- Take numbers from `--profile perf`. Compare paired, alternating runs by
  median, keep outliers, and say what a headless number leaves out (GPU upload,
  presentation).
- Follow reduced motion, animate properties rather than layout where possible,
  and never animate the first frame.

### Public history

Everything pushed stays public. Before committing, keep out home and temp paths,
machine and tailnet names, session IDs, private chat links, agent handoff notes,
raw logs and benchmark dumps, and generated media nothing links to. Write
repo-relative paths.

## File naming

- Never repeat a shared leading prefix across sibling files. When two or more
  files in one directory share a leading prefix, group them into a directory
  named after that prefix and drop the prefix from the filenames:
  `bulb-state.rs` and `bulb-token.rs` become `bulb/state.rs` and
  `bulb/token.rs`.
- Stacked prefixes collapse the same way, one directory per prefix segment.
- When the directory already names the concept, drop the redundant prefix in
  place: `components/bulb-card.rs` becomes `components/bulb/card.rs`.
- A file named exactly after the prefix (or its plural) stays as the entry file
  next to the group directory: `bulb.rs` next to `bulb/`.
- Cross-module imports use `crate::...` paths.

## Agent skills

### Issue tracker

Issues live in Linear (team JYOT, personal workspace), driven with the `rata`
CLI; PRs carry `JYOT-<n>` and `Closes JYOT-<n>`. See
`docs/agents/issue-tracker.md`.

### Triage labels

The five canonical triage roles use their default label strings. See
`docs/agents/triage-labels.md`.

### Domain docs

Single-context: `GLOSSARY.md` and `docs/adr/` at the repository root. See
`docs/agents/domain.md`.
