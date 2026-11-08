# Fonts

jyoti bundles three families so every machine draws the same interface:

| File | Family | Role | Source |
| --- | --- | --- | --- |
| `InterVariable.ttf` | Inter Variable (registered as `Inter`) | interface and tabular figures | <https://github.com/rsms/inter> |
| `Fraunces.ttf` | Fraunces | display names and page titles | <https://github.com/google/fonts/tree/main/ofl/fraunces> |
| `IBMPlexMono-Regular.ttf` | IBM Plex Mono | addresses, hex values, paths | <https://github.com/google/fonts/tree/main/ofl/ibmplexmono> |

Each family is licensed under the SIL Open Font License 1.1. The licence text
sits next to it:

- `OFL-Inter.txt`
- `OFL-Fraunces.txt`
- `OFL-IBMPlexMono.txt`

The fonts are loaded through `AppSpec::fonts` in `src/main.rs`; the family
names come from each file’s name table (`Inter Variable`, `Fraunces`, `IBM Plex
Mono`).
