# jyoti

A terminal control panel for Xiaomi light bulbs. One Bun process runs an OpenTUI (Solid) interface and speaks Xiaomi's miIO protocol to the bulbs over the local network. There is no daemon and no client/server split. `README.md` documents the product, `GLOSSARY.md` the domain vocabulary.

## Workspace

One Bun package at the repository root:

- `src/main.tsx` — the process entry point; it renders the Solid app.
- `src/components/` — Solid UI components.
- `src/lib/` — domain and protocol code (miIO packets, bulb state, config parsing, screen capture and colour reduction) and shared helpers.
- `native/` — the C11 screen-capture library, the vendored Wayland protocol XML and `build.sh`. It is not part of the Bun package's TypeScript.

Inside `src`, import through the `@/*` alias. `tsconfig.json` holds the compiler options.

## Commands

- `bun start` — run the TUI.
- `bun run dev` — run the TUI in watch mode.
- `bun run build:native` — build the C11 capture library with `cc`, `pkg-config` and `wayland-scanner`.
- `bun run check` — format check, type-aware oxlint (including the Effect rules) and `tsc`. Run it before committing.
- `bun run format`, `bun run lint`, `bun run lint:fix`, `bun run tsc`.

## Conventions

- Bun is the runtime and package manager; ESM throughout.
- No barrel files and no re-exports: import the leaf module directly.
- No lint rule suppressions; fix the code instead of disabling a rule.
- No `as any`, no casts that silence the type checker, no non-null assertions, no `@ts-ignore`.
- Comments only to explain a hard "why this way?"; code says what it does.
- The app owns its state: there is no daemon, no database and no HTTP API.
- Bulb tokens live in the OS keyring (Secret Service); never commit them, write them to config or log them. Metadata lives in `$XDG_CONFIG_HOME/jyoti/bulbs.json`.
- Wire values follow the miIO schema used by `bulbs.py`: `siid`/`piid` properties, integer brightness, kelvin temperature and packed RGB. Do not copy Python into this repository.
- Do not add a dependency unless the task needs it.
- Do not add tests. Verify changes with `bun run check` and by running the app.

### Effect

The app runs Effect 4.0.7:

- Services: `class X extends Context.Service<X, XShape>()("jyoti/lib/<area>/<file>/X")` with `static readonly layer = Layer.effect(X, Effect.gen(function* () { … }))` that returns `X.of({ … })`.
- Service methods and other reusable effects: `Effect.fn("Domain.operation")(function* …)`.
- Errors: `Schema.TaggedError`.
- Records: name the schema `XSchema` and its decoded type `X`, then export both from the leaf module.
- Runtime configuration goes through `Config`, not `process.env`.

The recommended Effect lint rules come from `@effect/tsgo` through `oxlint.config.ts`, so `bun run lint` reports Effect diagnostics without a language-server install. `node_modules/effect/AGENTS.md` resolves from the repository root.

## File naming

- Never repeat a shared leading prefix across sibling files. When two or more files in one directory share a leading hyphenated prefix, group them into a directory named after that prefix and drop the prefix from the filenames: `bulb-state.ts` and `bulb-token.ts` become `bulb/state.ts` and `bulb/token.ts`.
- Stacked prefixes collapse the same way, one directory per prefix segment: `advertisement-lead-chat-inbox-rows.ts` becomes `advertisement/lead/chat/inbox/rows.ts`.
- When the directory already names the concept (singular or plural), drop the redundant prefix in place: `components/bulb-card.tsx` becomes `components/card.tsx` inside `components/bulb/`.
- A file named exactly after the prefix (or its plural) stays as the entry file next to the group directory: `bulb.ts` next to `bulb/`.
- Solid hook files keep the `use-` prefix; only the domain prefix after it is grouped or dropped: `use-bulb-power.ts` becomes `bulb/use-power.ts`.
- Cross-directory imports use the `@/*` alias.

## Agent skills

### Issue tracker

Issues live in Linear (team JYO, personal workspace), driven with the `rata` CLI; PRs carry `JYO-<n>` and `Closes JYO-<n>`. See `docs/agents/issue-tracker.md`.

### Triage labels

The five canonical triage roles use their default label strings. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: `GLOSSARY.md` and `docs/adr/` at the repository root. See `docs/agents/domain.md`.
