# 10. Sync controls live in one panel, not in global keys

Date: 2026-10-29

Status: Accepted

## Context

Screen sync grew controls one hotkey at a time: `+` and `-` for the gain, `x` for pause, `m` for the monitor, and a second line of key hints in the strip and the bar. The owner asked for a help surface that shows the active keys, not more global bindings. The bar became the documentation, and the controls were hidden behind keys that only worked in the right context.

## Decision

- `s` opens one **Sync panel**. The panel owns start/stop, pause/resume, the captured monitor and the sync brightness gain.
- `↑` `↓` move between rows, `←` `→` adjust, `enter` acts, `esc` closes.
- The global `+`, `-`, `=`, `x` and `m` bindings are removed. The key bar carries only top-level actions.
- `?` opens a contextual help panel that lists the keys active in the current context. When sync is running, the light and bulb-editing keys that sync blocks are not listed.
- The sync strip keeps status only; it no longer carries a key line.
- Saturation and white mix, when added, belong in the same panel; they do not become global keys.

## Consequences

- Starting or stopping sync is one extra key: `s`, then `enter`. Adjusting the gain is `s`, `↓` to the gain row, `←` or `→`.
- Every sync control has one visible home. The bar and strip stop growing with each new control.
- The help panel is the discovery surface; it must be kept in step with the active context.
- ADR 0009's strip and toast key hints are superseded; its persistence and neutral defaults still hold.
- No behaviour of the sync engine changes: the panel calls the same start, pause, monitor and gain operations.
