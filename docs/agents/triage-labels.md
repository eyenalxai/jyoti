# Triage Labels

The skills speak in terms of five canonical triage roles. This file maps those
roles to the label strings used in team JYO.

## State roles

| Label in mattpocock/skills | Label in our tracker | Meaning                                  |
| -------------------------- | -------------------- | ---------------------------------------- |
| `needs-triage`             | `needs-triage`       | Maintainer needs to evaluate this issue  |
| `needs-info`               | `needs-info`         | Waiting on reporter for more information |
| `ready-for-agent`          | `ready-for-agent`    | Fully specified, ready for an AFK agent  |
| `ready-for-human`          | `ready-for-human`    | Requires human implementation            |
| `wontfix`                  | `wontfix`            | Will not be actioned                     |

## Category roles

| Label in mattpocock/skills | Label in our tracker | Meaning                       |
| -------------------------- | -------------------- | ----------------------------- |
| `bug`                      | `Bug`                | Something is broken           |
| `enhancement`              | `enhancement`        | New feature or improvement    |

`Bug` is the workspace-level label (capitalized); do not create a second `bug`.

A live, triaged issue carries exactly one category role and one state role.
