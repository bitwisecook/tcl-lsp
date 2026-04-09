# KCS: Authoring iRule scripts for examples and tests

## Goal

Create iRule scripts that are realistic enough for diagnostics and flow analysis without introducing unrelated complexity.

## Checklist

- Scope each script to one event flow story (`CLIENT_ACCEPTED`, `HTTP_REQUEST`, etc.).
- Keep side effects explicit (`pool`, `node`, `HTTP::respond`, `reject`).
- Include both positive and negative paths so event-flow checks can reason about alternatives.
- Add comments that explain intent of each event block.

## Recommended templates

1. Minimal single-event rule for syntax/command checks.
2. Two-event rule to validate cross-event variable flow.
3. Rule with security-sensitive sinks (headers, redirects, Tcl `exec`-like paths) for taint checks.

## Naming guidance

- `event_<event>_<behaviour>.irul`
- `flow_<source>_to_<sink>.irul`

## Storage guidance

- Put reusable analyser fixtures under `samples/for_screenshots/` or dedicated `tests/fixtures` subfolders.
- Keep generated or third-party corpora separated from hand-curated canonical fixtures.
