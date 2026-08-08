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

### CI results are not a substitute for macOS results

Worse than a constant factor: the two platforms take **different code
paths**. Measured on the same corpus, same four documents, same 6 requests,
same 25 locations returned:

| check | Linux (CI) | macOS (arm64) |
|---|--:|--:|
| `nav.references` | 0.38 s / **0.0 CPU s** | 9.97 s / **9.9 CPU s** |
| `lens.resolve` | 0.21 s / 0.0 CPU s | 9.80 s / 9.8 CPU s |
| `nav.after_rename` | 0.15 s / 0.0 CPU s | 9.79 s / 9.8 CPU s |

Identical answers, ~10 CPU-seconds each on macOS and effectively none on
Linux. The #1297 blowup (`RunOrder::alternatives` losing its fast path) does
not trigger on the Linux runner. `nav.hover` differs in *result* too — 6
items on Linux, 0 on macOS — so this is not only a performance divergence.

Consequence: **the CI job cannot catch a #1297-class regression**, and the
release-notes graphs, if generated on a Linux runner, will not show the
behaviour macOS users get. Until that is understood, treat CI as a trend
line for the checks it does exercise, and run the sweep on macOS before
trusting a release figure. This is tracked on #1297.

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

## Reading the pre-2.1.14 results

Every version from **2.1.0 to 2.1.13 fails `scan.workspace`** with an
identical 120.0 s. That figure is *this harness's wait timing out*, not the
server's indexing time: `wait_for_workspace_scan` blocks on a
workspace-scan-complete signal that those releases do not emit. 2.1.14 is
the first version where it settles (1.8 s), and 2.1.15/2.1.16 do too
(0.4 s / 0.8 s).

Two consequences when reading the graphs:

* The 120 s `scan.workspace` bars for 2.1.0–2.1.13 are hatched (failed) and
  must not be read as "indexing took two minutes". They are a harness
  limitation against older servers.
* Those versions were still indexing while the rest of the suite ran, so
  their later checks measure a partially-populated index. The `items`
  column shows it plainly — `nav.references` returns **7** results on
  2.1.0–2.1.9, **13** on 2.1.10, and **25** from 2.1.11 onward, which is
  what the settled versions also return. A fast check against a smaller
  index is not a faster server.

So treat 2.1.0–2.1.9 as indicative only. From 2.1.11 the reference counts
agree with the modern versions, and from 2.1.14 the scan settles too, which
is where the numbers become straightforwardly comparable.

## Corpus scopes

`python3 fetch_corpus.py --list` prints the groups and scopes; the manifest is
the source of truth.

`small` (georgtree, ~113 files) is the default and the right one for routine
release tracking: seconds to run, and it still contains `SpiceGenTcl`, the
smallest corpus member that builds a `source` forest and package edges at
once — the shape behind #1297. `medium` and `full` (~2400 files) exist for
scan-cost and steady-state memory work.

`irules` (14 repos, 217 source files) is the iRules corpus. It is **not** a
benchmark scope — the check suite's anchors and seeded document selection are
tuned for the Tcl corpus — but it is the right one for dialect work:
diagnostics, the command registry, `when` blocks. `everything` is both
dialects, for sweeps that want maximum surface (fuzzing, parser crash hunts,
false-positive audits) rather than comparable timings.

Two things to know before sweeping the iRules tree:

- **iRule sources are often not `.tcl`.** Across these repos it is roughly 147
  `.tcl`, 63 `.irule`, 7 `.tmsh` and 198 `.txt` — DevCentral publishes iRules
  as plain text. Globbing `*.tcl` silently drops most of the corpus, so
  `fetch_corpus.py` counts `.tcl`/`.irule`/`.tmsh` and reports which suffixes
  it counted.
- **The iRules groups are outside every existing benchmark scope, on
  purpose.** Stored results only compare within one `corpus.revision`, so
  adding files to `small`/`medium`/`full` would invalidate every measurement
  taken so far. No existing pin moved when they were added, so
  `corpus.revision` did not change either.

`everything-private` additionally pulls `bitwisecook/tcl-lsp-testsrc` (~620
publicly-sourced iRules gathered from 60+ upstream repos). That repo is
private, so the entry needs `--include-private` and credentials; without them
the fetcher prints a `skipped` line rather than failing, because a private
entry quietly dropped from a corpus is indistinguishable from one that was
never there.

## Growing the corpus

Add a `[[repo]]` block with an exact `commit` and put its `group` in a scope.
Pin to a SHA, never a branch — the whole point is that a run a year from now
measures the same bytes. Advancing an *existing* pin is a different act: it
invalidates every stored benchmark result, so bump `corpus.revision`, re-run
the sweep, and say so in the release notes.

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
