# jyoti

A terminal control panel for Xiaomi light bulbs. OpenTUI (Solid) draws the interface, and the same Bun process talks to the bulbs directly over the local network with Xiaomi's miIO protocol.

> **Status: scaffold only.** The app shell, tooling and documentation are in place. Token loading, the miIO client and the bulb controls are not implemented yet.

## How it will work

- One process. The TUI is the app; there is no daemon, HTTP API or database.
- Bulbs are addressed by IP and token, read from `~/.local/share/xiaomi-bulbs/tokens.env` — the same file `bulbs.py` already uses.
- Status reads go through miIO `get_properties`; changes go through `set_properties` over UDP port 54321.
- The UI is planned to expose power, brightness, color temperature and RGB color, for one bulb, a named target or all bulbs.

## Requirements

- Bun 1.4 or newer
- Bulbs on the same LAN, with local control enabled and tokens in the file above

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

Press `q` or `ctrl+c` to quit.

## Layout

```
src/
  main.tsx            process entry point
  components/app.tsx  application shell
  lib/theme.ts        color palette
```

UI components live in `src/components/`; domain and protocol code will live in `src/lib/`. Imports inside `src` use the `@/*` alias.

## Documentation

- `AGENTS.md` — working agreements for this repository.
- `GLOSSARY.md` — the language used for bulbs, tokens and miIO.
- `docs/adr/` — decisions, starting with the direct-connection architecture.

## License

MIT
