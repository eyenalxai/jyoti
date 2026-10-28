# 2. Bulb credentials come from tokens.env

Date: 2026-10-28

Status: Accepted

## Context

`bulbs.py` already reads `~/.local/share/xiaomi-bulbs/tokens.env` and the setup script already writes it. The file holds the IP, token, name and model for each bulb. Introducing a second configuration file, a keyring entry or a database would duplicate that source of truth and make the existing setup flow stop working.

## Decision

`jyoti` reads the same file with the same `BULB_<n>_*` keys. A missing file is reported as a startup error that names the path. The app never writes secrets.

## Consequences

- The existing `setup-tokens.sh` flow keeps working unchanged.
- The tokens file stays outside the repository and outside logs.
- A user who moves the file needs to update the default path, which can later become a command-line flag.
