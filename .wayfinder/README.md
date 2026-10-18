# Local wayfinder tracker

No remote issue tracker (this effort is not a herdr maintainer repo), so the map
lives here as markdown.

- `map.md` — the map (label `wayfinder:map`).
- `tickets/NN-slug.md` — child tickets. Frontmatter fields:
  - `title`, `type` (`research` | `prototype` | `grilling` | `task`)
  - `status`: `open` | `closed`
  - `assignee`: empty = unclaimed; set before work = claim
  - `blocked_by`: list of ticket files; unblocked when all are `closed`
- Resolution: append `## Resolution` to the ticket, set `status: closed`, add a
  line to the map's Decisions so far.
- `research/` — research findings (stand-in for throwaway `research/<name>`
  branches; there is no remote to push to).

Frontier = `status: open`, empty `assignee`, every `blocked_by` closed.
