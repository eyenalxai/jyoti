# 1. The TUI talks to the bulbs directly

Date: 2026-10-28

Status: Accepted

## Context

`vingroto` separates a daemon from stateless clients because mail needs a shared cache, keyring credentials, persistent connections and work that continues while the interface is closed. `jyoti` controls up to three bulbs on the local network. It has no shared cache, no credentials store beyond a local token file, no background sync and no second client. A daemon would add a second process, a wire protocol and lifecycle handling without a user-visible benefit.

## Decision

`jyoti` is one Bun process. The OpenTUI (Solid) interface calls the miIO client in-process, and the client sends UDP packets to each bulb directly.

## Consequences

- Closing the TUI stops all activity; there are no scheduled or background jobs.
- Bulb state is read on demand and is not cached across runs.
- The miIO client must be safe to call from the UI event loop: socket work is asynchronous and bounded by a timeout.
- A second client, background schedules or a shared cache would justify revisiting this decision.
