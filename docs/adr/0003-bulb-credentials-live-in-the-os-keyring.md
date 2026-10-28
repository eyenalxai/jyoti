# 3. Bulb credentials live in the OS keyring

Date: 2026-10-28

Status: Accepted

## Context

ADR 0002 made `tokens.env` the only source of bulb credentials and said the app never writes secrets. The panel now adds, edits and removes bulbs itself, so the app has to save tokens. Writing them into a plain config file would put a reusable device key on disk in cleartext, next to metadata that is not secret.

The OS keyring is already available on the target desktop through Secret Service and `secret-tool`. It can store a token per bulb, keyed by the bulb's stable ID, while the non-secret name, address and model live in a small XDG config file.

## Decision

- Each bulb's token is stored in the Secret Service under the `jyoti` service and the bulb ID as the account.
- Name, IP address and optional model are stored in `$XDG_CONFIG_HOME/jyoti/bulbs.json`, falling back to `~/.config/jyoti/bulbs.json`. The file is written atomically with mode `0600`.
- The app writes and deletes keyring entries. A token is read only when a command needs it.
- The app starts empty. It does not import or write `tokens.env`; that file stays the reference script's input.
- Token values never enter the config file, a log line or an error message. Errors name the bulb or its address.

## Consequences

- The app requires `libsecret-tools` and an unlocked Secret Service keyring. A missing or locked keyring is reported as a keyring error, not a generic failure.
- Moving to another machine means re-entering each bulb's token; the metadata file can be copied, the tokens cannot.
- `bulbs.py` keeps working from `tokens.env` and is now independent of jyoti's storage.
- If a config write fails after a keyring change, the previous token is restored; an orphaned token is not created.
