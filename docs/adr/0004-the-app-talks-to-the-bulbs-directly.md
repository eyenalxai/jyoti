# 4. The app talks to the bulbs directly

Date: 2026-11-08
Status: Accepted

## Context

jyoti controls a small number of bulbs on the local network. It has no shared
cache, no background sync while closed and no second client. A daemon would add
a second process, a wire protocol and lifecycle handling without a user-visible
benefit.

## Decision

One Rust process owns the window, the bulb records, one miIO worker per bulb and
the screen-sync thread. The workers speak UDP directly to the bulbs. There is no
daemon, no database and no HTTP API. Bulb operations are addressed independently
so one slow bulb does not block the others.

## Alternatives

- A daemon plus stateless clients: rejected because there is no shared state or
  background work to justify it.
- A local HTTP API: rejected because the app is the only client.
- One global worker for every bulb: rejected because a slow bulb would delay the
  others.

## Consequences

The app owns its state and exits completely. A sync run stops with the app; there
is no background controller after the window closes. Each bulb has its own
worker and session, so a failure is isolated to that bulb.
