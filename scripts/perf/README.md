<!--
tcl-lsp — a language server and toolchain for Tcl
Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
SPDX-License-Identifier: AGPL-3.0-or-later
-->

# Cross-version performance benchmark

A deterministic suite that drives a real `tcl-lsp-server` through a fixed
editing session against a pinned corpus of public Tcl projects, records
memory / CPU / wall time, and renders graphs for the release notes.

It exists to answer one question per release — *did this get worse?* — with
evidence rather than impression. It was built after a memory-leak review
against the issue #1181 corpus, and its first output showed v2.1.14 holding
479 MiB where v2.1.15 holds 170 MiB.

## Quick start

```bash
cd scripts/perf
python3 fetch_corpus.py --scope small          # pinned clones into ./corpus
python3 get_server.py --tag v2.1.15            # binary into ./servers
python3 bench.py --server servers/v2.1.15/tcl-lsp-server --version 2.1.15 --scope small
python3 report.py                              # ./graphs/{memory,cpu,walltime}.svg + summary.md
```

Whole sweep:

```bash
python3 fetch_corpus.py --scope small
for tag in $(python3 get_server.py --all | awk '$2 != "UNAVAILABLE" {print $1}'); do
  python3 bench.py --server "servers/$tag/tcl-lsp-server" --version "${tag#v}" --scope small
done
python3 report.py
```

## The pieces

| file | role |
|---|---|
| `MANIFEST.toml` | Pinned corpus commits, benchmark scopes, anchor files, version list. The single source of truth. |
| `fetch_corpus.py` | Reconstitutes the corpus at its pins. `--verify-only` checks for drift without network. |
| `get_server.py` | Resolves a tag to a binary: published release asset first, else a build from the tag in a throwaway worktree. Cached. |
| `bench.py` | Runs the 15-check suite against one binary, samples RSS/CPU every 250 ms, writes `results/<version>.json`. |
| `report.py` | Turns `results/*.json` into three SVGs plus `summary.md`. Stdlib only, byte-for-byte deterministic. |

`results/` and `graphs/` are committed — they are the record. `corpus/`,
`servers/`, `.stage/` and `.build/` are generated and ignored.

## The check set

Ordered, hard-coded, and an **interface**: reordering or redefining a check
invalidates comparison against stored results. Add checks at the end.

| # | id | what it measures |
|--:|---|---|
| 1 | `scan.workspace` | Cold cross-file index build |
| 2 | `open.docs` | Opening 4 documents + first analysis |
| 3 | `edit.storm` | 400 keystrokes, no requests — pure buffer/analysis churn |
| 4 | `edit.tokens` | 100 keystrokes each followed by `semanticTokens/full` |
| 5–9 | `nav.*` | hover, definition, references, completion, symbols+folding |
| 10 | `lens.resolve` | `codeLens` + `codeLens/resolve` (where reference counts are computed) |
| 11 | `action.codeaction` | Code actions at 12 positions, as an editor issues on cursor move |
| 12 | `refactor.rename` | Workspace symbol rename |
| 13 | `refactor.filerename` | External file rename: `willRenameFiles` → move → `didRenameFiles` → watched-file events |
| 14 | `nav.after_rename` | References again — a twin of #7, so state stranded by a rename shows as a gap between them |
| 15 | `close.docs` | Closing every document (closed-file retention) |

## What makes it deterministic, and what does not

Deterministic: the corpus (pinned commits, verified), the documents chosen
(sorted by relative path, then shuffled with a fixed seed), the positions
navigated (definition sites, in source order), the edits (fixed sequence of
constant-length inserts), the check list and its order, and every byte
`report.py` emits.

**Not** deterministic, and not fixable: wall time and CPU depend on the
machine. Run the whole sweep on one host, or the bar graph compares hardware
instead of releases. `summary.md` records the host it ran on for this reason.

Two design points worth knowing:

- **Typing-storm texts never repeat and never grow.** Each keystroke produces
  a fresh text of constant length, so nothing can hide behind an
  identical-input cache short circuit, and growth can never be blamed on a
  larger buffer. A version whose memory line keeps climbing to the right of
  this check has a leak; one that plateaus does not.
- **Anchor files.** `[anchors]` in the manifest lists paths that must always
  be opened, pinning known pathological shapes. Without one, a seeded
  selection that happens to land on four cheap files reports a clean run
  while the regression the suite exists to watch sits untouched — this
  actually happened during development: `nav.references` read 0.013 s until
  the #1297 anchor was added, then 4.0 s. **Add an anchor whenever a
  performance issue is found.**
- **A check that issues zero requests fails.** A 0.000 s bar in a release-notes
  graph reading as "this got faster" is worse than no bar, so the suite
  refuses to record one.

## Corpus scopes

`small` (georgtree, ~113 files) is the default and the right one for routine
release tracking: seconds to run, and it still contains `SpiceGenTcl`, the
smallest corpus member that builds a `source` forest and package edges at
once — the shape behind #1297. `medium` and `full` (~2400 files) exist for
scan-cost and steady-state memory work.

## Version coverage

Native `tcl-lsp-server` release assets start at **v2.1.5**. v2.1.0–v2.1.4
have no asset and are built from their tags, which is slow on first run and
cached after. v2.1.2 was never released. `get_server.py --all` prints
`UNAVAILABLE` for anything it cannot produce rather than silently skipping.

Prefer the published asset over a local rebuild where one exists: it is the
binary users actually ran, and a rebuild can differ in toolchain and flags.

## Reading the graphs

- `memory.svg` — RSS against elapsed time, one line per version. A line that
  climbs and never flattens is the leak shape. Lines end at different x
  values because slower versions take longer to complete the same suite.
- `cpu.svg` — CPU utilisation, differentiated from cumulative CPU seconds.
  Can exceed 100%: the server is multi-threaded.
- `walltime.svg` — grouped bars, log10 Y by default because checks span
  0.001 s to 30 s and a linear axis makes the fast ones invisible. Hatched,
  dashed bars are checks that failed or timed out; their height is the
  timeout budget, not a measurement.
