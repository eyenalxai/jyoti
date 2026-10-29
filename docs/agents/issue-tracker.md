# Issue tracker: Linear

Issues and specs for this repository live in Linear, team **JYOT** (`Jyoti`) in
the **personal** workspace. Use the `rata` CLI for all operations. GitHub Issues
is not the tracker.

## Repository link

The repository config lives in rata's local database, not in a committed file.
On a fresh machine, bind it once from the repository root:

```sh
rata link --workspace personal --team JYOT
```

The link is per clone, so every worktree of this repository shares it. When
`LINEAR_API_KEY` is set, rata uses that key and keeps the recorded workspace.

## Conventions

- **Create**: `rata issue create --title "..." --body-file - --assignee me`.
- **Read**: `rata issue show <ref> --comments --json`.
- **List**: `rata issue list --team JYOT` with the filters you need: `--state`,
  `--state-type`, `--label`, `--assignee`, `--parent`, `--text`, `--limit`.
- **Comment**: `rata issue comment create <ref> --body-file -`.
- **Apply / remove labels**: `rata issue label add <ref> <label...>` and
  `rata issue label remove <ref> <label...>`.
- **Close**: `rata issue close <ref> --comment "..."`.
- **Search**: `rata search "..."`.
- **Set a priority**: `rata issue update <ref> --priority <value>`.
- **Assign**: `rata issue assign --assignee me <ref>`. Create issues with `--assignee me`.

An issue reference accepts an identifier (`JYOT-42`), a UUID, or a `linear.app`
issue URL. List commands return one page; continue with
`--after <pageInfo.endCursor>` while `pageInfo.hasNextPage` is true.

## Pull requests

Issues describe the work; PRs deliver it. Linear's GitHub integration links a PR
to its issue from the branch name, the PR title, or the PR body, and moves the
issue by the magic word used:

- **Branch**: `feat/JYOT-42-short-slug`. The issue ID in the branch name links
  the PR automatically.
- **Title**: `<type>(<scope>): <summary>`, with no issue ID. The branch name
  and the body carry the ID; keep the title clean.
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
delegated child returns the PR URL, head SHA, and check state to its parent.

## When a skill says "publish to the issue tracker"

Create a Linear issue with `rata issue create`, set its priority, and assign it
to the owner (`--assignee me`). A spec is an issue too: do not create a project
and do not put `Spec:` in the title; its tickets are its child issues.

## When a skill says "fetch the relevant ticket"

Run `rata issue show <ref> --comments`.

## Specs and tickets

A **spec** is an issue that holds a spec. Its tickets are its **child issues**.

- **Publish a spec**: `rata issue create --title "<title>" --body-file -`.
- **Publish the tickets**: `/to-tickets` creates each ticket as a child issue:
  `rata issue create --title "..." --parent <spec-ref> --label ready-for-agent --body-file -`.
  Wire blocking edges with `rata issue link <ticket-ref> --blocked-by <blocker-ref>`.
- **Implement the spec**: `/implement-spec` reads the whole set with
  `rata issue list --parent <spec-ref> --limit 250 --json`, paging with
  `--after` while `pageInfo.hasNextPage` is true.
- **Close the spec**: when every ticket is Done, close it with
  `rata issue close <spec-ref>` and a comment that points at the result.

## Priorities

Every issue carries a priority:

- `urgent` — breaks production or blocks a release.
- `high` — blocks other work.
- `medium` — the fallback when no other value applies.
- `low` — nice-to-have.

## Wayfinding operations

Used by `/wayfinder`. The **map** is a single issue labelled `wayfinder:map`,
with **child** issues as tickets.

- **Map**: `rata issue create --title "..." --label wayfinder:map --body-file -`.
- **Child ticket**: `rata issue create --title "..." --parent <map-ref> --label wayfinder:<type> --body-file -`.
  Types: `wayfinder:research`, `wayfinder:prototype`, `wayfinder:grilling`,
  `wayfinder:task`.
- **Blocking**: `rata issue link <child-ref> --blocked-by <blocker-ref>`, or the
  inverse with `--blocks`.
- **Frontier query**: `rata issue list --parent <map-ref> --unblocked --unassigned --json`.
- **Claim**: `rata issue assign me <ref>`, the session's first write.
- **Resolve**: comment, close, then append a context pointer to the map's
  Decisions-so-far with `rata issue update <map-ref> --body-file -`.
