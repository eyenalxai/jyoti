# Issue tracker: Linear

Issues and specs for this repository live in Linear, team JYOT (`Jyoti`) in the
`personal` workspace. Use the `rata` CLI for every tracker operation. GitHub
Issues is not the tracker.

Load the rata skill before the first tracker command. It holds the canonical
forms, the read loop and the write loop. This file holds the repository facts.

## Repository link

The repository link lives in rata's local database, not in a committed file. On
a fresh machine, bind it once from the repository root:

```sh
rata link --workspace personal --team JYOT
```

The link is per clone, so every worktree of this repository shares it. When
`LINEAR_API_KEY` is set, rata uses that key and keeps the recorded workspace.
Run `rata status --json` to read the team, the issue defaults and the viewer.

When the stored team key is stale, a read warns and repairs the link. When the
key no longer exists, the read fails with `not_found`; run `rata team list`, then
`rata link --workspace personal --team JYOT`.

## Labels

Every live issue carries one category label and one state label. The category
labels are the workspace-level `bug` label and the team `enhancement` label. The
state labels are `needs-triage`, `needs-info`, `ready-for-agent`,
`ready-for-human` and `wontfix`. See `docs/agents/triage-labels.md` for the role
mapping.

Apply and remove labels with `rata issue label add <ref> <label...>` and
`rata issue label remove <ref> <label...>`.

## Priority and assignee

Every issue carries a priority: `urgent`, `high`, `medium` or `low`. Every
actionable issue is assigned to the owner. Pass `--priority` and `--assignee me`
on create; change them later with
`rata issue update <ref> --priority <value>` and
`rata issue assign <ref> --assignee me`.

Wayfinder maps and children are the exception: they stay unassigned until the
claim step and carry no triage labels. Create them with `--no-default-labels`
and `--assignee none`.

## Specs and tickets

A spec is an issue. Do not create a project for it and do not put `Spec:` in the
title. Its tickets are its child issues.

- **Publish a spec**: `rata issue create --title "<title>" --label ready-for-agent --priority medium --assignee me --body-file -`.
- **Publish the tickets**: add `--parent <spec-ref>` and the same label,
  priority and assignee to each create.
- **Wire blocking edges**: `rata issue link <ticket-ref> --blocked-by <blocker-ref>`.
- **Read the set**: `rata issue list --parent <spec-ref> --all --json --include relations`.
- **Read known tickets**: `rata issue list --id <ref> --id <ref> --json`.
- **Start a ticket**: `rata issue status <ticket-ref> --json --with-body`, then
  claim it with `rata issue assign <ticket-ref> --assignee me`.
- **Replace record sections**: `rata issue update <spec-ref> --sections-file -`
  with `{"sections":[{"heading":"...","content":"..."}]}`. Preview with
  `--dry-run`, then write with `--if-unchanged <updatedAt>`.
- **Close the spec**: when every ticket is Done, close the spec with
  `rata issue close <spec-ref> --comment "..."`.

## Wayfinding operations

A map is an issue labelled `wayfinder:map`. Its tickets are child issues with a
`wayfinder:<type>` label: `research`, `prototype`, `grilling` or `task`.

- **Map**: `rata issue create --title "..." --label wayfinder:map --no-default-labels --assignee none --body-file -`.
- **Child**: add `--parent <map-ref>` and the type label.
- **Frontier**: `rata issue list --parent <map-ref> --unblocked --unassigned --all --json --include relations --sort priority`.
- **Claim**: `rata issue assign <ref> --assignee me`.
- **Resolve**: `rata issue comment create <ref> --body-file -`, then
  `rata issue close <ref>`, then append the context pointer to the map's
  Decisions-so-far section with
  `rata issue section append <map-ref> "Decisions so far" --body-file -`.

## Pull requests

**PRs as a request surface: no.** Linear holds issues only. When this
repository treats external pull requests as feature requests, triage them where
they live and mirror the outcome here.

Issues describe the work; PRs deliver it. Linear's GitHub integration links a PR
to its issue from the branch name, the PR title, or the PR body, and moves the
issue by the magic word used.

- **Branch**: `feat/JYOT-42-short-slug`. The issue ID in the branch name links
  the PR automatically.
- **Body**: `Closes JYOT-42` for a PR that completes the issue. Use
  `Refs JYOT-42` or `Part of JYOT-42` when the issue stays open.
- **One issue per PR.** When a PR really spans issues, name each one after
  `Closes`. Never put an issue ID you do not mean to touch in the branch name:
  the branch name links the PR even if the body does not.

Closing magic words: `close`, `closes`, `closed`, `closing`, `fix`, `fixes`,
`fixed`, `fixing`, `resolve`, `resolves`, `resolved`, `resolving`, `complete`,
`completes`, `completed`, `completing`, `implement`, `implements`,
`implemented`, `implementing`, `linear issue`. Non-closing: `ref`, `refs`,
`references`, `part of`, `contributes to`, `toward`, `towards`. Relation only:
`relates to`, `related to`.

`.github/pull_request_template.md` carries the `Closes JYOT-` slot.

## GitHub integration

The Linear GitHub app must have access to `eyenalxai/jyoti`. Status automation
is a team setting: **Settings → Teams → JYOT → Workflows & automations → Pull
request and commit automations**. The intended mapping:

| PR event              | Issue state   |
| --------------------- | ------------- |
| opened or reopened    | `In Progress` |
| merged (closing word) | `Done`        |

A non-closing word still moves the issue on other events but skips the merge
destination. A relation word links the PR without changing status.

Autolink references are configured on the repository, so `JYOT-42` written in a
PR, issue, comment, or commit renders as
`https://linear.app/utbetdix/issue/JYOT-42`.

Linear connects one GitHub account to one Linear workspace. The JYOT team lives
in the personal workspace; if the GitHub app is already bound to another
workspace, the integration cannot be enabled here.

A T3 Code thread registers every PR it creates or picks up with
`link_pull_request` and the full PR URL. For a stack, register every layer. A
delegated child returns the PR URL, head SHA and check state to its parent.
