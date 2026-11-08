# 3. Bulb tokens live in the OS keyring

Date: 2026-11-08
Status: Accepted

## Context

The app adds, edits and removes bulbs itself, so it has to save tokens. A token
is a reusable device key. Writing it into a plain config file would put it on
disk in cleartext beside metadata that is not secret.

## Decision

Tokens live in the OS keyring through the `keyring` crate: Secret Service on
Linux, Keychain on macOS and Credential Manager on Windows. The keyring service
is `jyoti`; the account is the bulb's stable UUID. Non-secret metadata lives in
`$XDG_CONFIG_HOME/jyoti/bulbs.json`. Legacy `secret-tool` entries are read and
migrated on first use.

## Alternatives

- Store tokens in the config file: rejected because it puts a reusable device key
  on disk in cleartext.
- Store tokens in an environment file: rejected because the app manages bulbs
  itself and a file is easy to leak.
- Use only the kernel keyring: rejected because it does not survive a reboot and
  does not exist on macOS or Windows.

## Consequences

A token never enters the config file, a log line or an error message. The app
needs an unlocked Secret Service keyring on Linux. A missing token is reported
inline and can be re-entered through Edit.
