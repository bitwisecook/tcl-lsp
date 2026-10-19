# Slow-test review, 2026-10-19

Scope: 206 tests that took 10 s or more (about 10,500 s of the 13,600 s measured), from capture47 (one process per test, serial, debug build) and Sol-26's integration45 runs (three-way parallel). Reviews are static (no new runs): luna-29 (core/compiler/db, 47 tests), luna-30 (server, 54), luna-31 (explorer, 105). User direction: user-decisions-20261019T1000Z.json.

## Shared product costs (fixing these needs no assertion change)

| Cause | Tests | Seconds | Fix |
|---|---|---|---|
| Bundled EDA spec packs (2 MB) parsed per process / per reload in debug, ~17-19 s each | ~120 (105 explorer, 14 server) | ~2,700 | memoise parsed packs; measure the load; dev-profile opt-level for hot crates or lazy per-pack load |
| Test backend scans the host Tcl install (/usr/lib, /usr/share/tcltk) on every workspace scan, surveying each index six times | 27 server (all 180 s timeouts) | ~4,860 | test harness presets discovered_tcl; product caches the per-directory survey across rescans |
| Nested bodies re-cut at every level (parse cut over whole nested text) | 7 | ~916 | Sol-28's memo (184 s -> 8.7 s on the nested wrapper, same results) |
| Interprocedural cost roughly quadratic in procedure count | 8 | ~466 | re-run/summary work; one counter test to confirm |
| Unversioned "tcl" Logical path 4-7 s per tiny document; per-query whole-document recapture | 22 | ~456 | product |
| Read-only requests analyse every document fresh per request (workspace symbols, call hierarchy, lens) | Server108 and others | - | reuse analysis |

## Defects found by the review

- lsp-core MAX_SCOPE_WALK_DEPTH and declaration MAX_BODY_DEPTH are 256 while the compiler cap is 64; an unguarded scope walk overflows near 100 levels.
- default_dialect_change_retries_after_concurrent_edit_1854 hangs: awaits did_change unbounded while holding the default_dialect guard.
- Explorer: 73 of 140 tests fail at HEAD, 41 because run_pipeline builds without the document analysis input, 19 because proc/method/when bodies lower as plain calls.

## Tier proposal

All originals stay in deep (CI and `make test-rust` before a PR) with assertion text unchanged. A new local, non-gating quick tier (nextest `[profile.quick]`, `make test-quick`) excludes a deep-only manifest checked by an xtask drift gate. Light versions are new tests that run in both. Performance budgets are work counters (analyses per request, pack parses per reload, scaling between two small inputs), not wall-clock limits. Proposed totals: quick tier about 230 s for core/compiler/db and under 1 min for explorer; server ~6,400 s -> ~420 s and explorer ~1,900 s -> ~185 s after the systemic fixes.
