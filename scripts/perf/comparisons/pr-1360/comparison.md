# PR #1360 full-corpus performance comparison

Base: `origin/rust` at `7c31a9d74` (including #1359). Candidate: `8246a30a0`. Both were built with the same Rust 1.97.0 toolchain and `cargo build --locked --release -p tcl-lsp-server` in isolated worktrees. The pinned #1181 `full` corpus contains 2,432 Tcl files from 21 repositories (corpus revision 1).

Six measured pairs followed discarded warm-ups. Pair order alternated base→candidate, then candidate→base. Every scenario used a fresh server and scratch corpus; no build, test, or other benchmark ran concurrently. Values are median ± median absolute deviation (min–max). Negative paired delta favours the candidate.

## Latency

| Scenario | rust base | PR candidate | Median paired delta | Candidate wins | Population |
|---|---:|---:|---:|---:|---|
| Cold workspace scan | 31.555 ± 0.524 s (30.994–34.277) | 36.068 ± 0.258 s (35.797–39.558) | +14.9% | 0/6 | all six pairs |
| Open 4 documents | 0.279 ± 0.019 s (0.252–0.326) | 0.307 ± 0.004 s (0.287–0.313) | +10.8% | 1/6 | all six pairs |
| Close 4 documents | 0.311 ± 0.005 s (0.305–0.320) | 0.316 ± 0.003 s (0.305–0.320) | +1.3% | 3/6 | all six pairs |
| 400-edit convergence | 0.731 ± 0.037 s (0.659–0.825) | 0.795 ± 0.040 s (0.748–1.021) | +7.2% | 2/5 | 5 successful pairs |

![Latency comparison](latency.svg)

## Resident memory

| Metric | rust base | PR candidate | Median paired delta | Candidate wins |
|---|---:|---:|---:|---:|
| Peak RSS | 971.312 ± 16.336 MiB (948.406–991.938) | 2933.297 ± 18.086 MiB (2876.812–2964.641) | +200.9% | 0/6 |
| Final RSS | 971.312 ± 18.430 MiB (945.547–991.938) | 2933.297 ± 17.953 MiB (2870.031–2960.438) | +200.9% | 0/6 |
| Net RSS growth | 965.109 ± 19.570 MiB (939.391–986.750) | 2927.625 ± 17.945 MiB (2863.875–2954.781) | +202.3% | 0/6 |

![Memory comparison](memory.svg)

## Reliability and complete deterministic suite

| Scenario | rust base | PR candidate | Interpretation |
|---|---:|---:|---|
| 100 light edits, semantic tokens after each | 0/6 | 1/6 | A failure is the first 30 s request timeout; both builds are unreliable at full-corpus scale. |
| 400-edit burst, then convergence request | 5/6 | 6/6 | Candidate completed every repetition; the base had one timeout. |
| Existing 15-check deterministic suite | 0/1 | 1/2 | Base was stopped after >690 s in `edit.tokens`. Candidate completed once in 49.2 s, then a measured repeat was stopped after >300 s at the same check. |

![Reliability comparison](reliability.svg)

The complete successful candidate run peaked at 2,957.0 MiB RSS. Its check wall times were: scan 35.901 s; open 0.297 s; heavy edit dispatch 0.651 s; light tokenised edits 9.234 s; hover 0.001 s; definition 0.007 s; references 1.188 s; completion 0.294 s; symbols/folding 0.193 s; lens 0.215 s; code actions 0.348 s; symbol rename 0.056 s; file rename 0.348 s; references after rename 1.162 s; close 0.313 s.

## Merge-strategy signal

The common semantic sidecar is not performance-neutral at full-corpus scale. It raises median cold-scan time and roughly triples resident memory while leaving document open/close latency unchanged. It improves heavy-burst convergence reliability in this sample, but light per-edit token requests remain unstable in both builds. The safest merge strategy is to split or gate eager semantic-sidecar construction before enabling it for every indexed function; the registry contracts, runtime ABI, and bounded backend plumbing can land independently from eager workspace materialisation.

## Raw evidence

`results/` contains all 36 measured scenario JSON files (six repetitions × two builds × three scenarios) plus the one successful full-suite candidate JSON. Each file includes per-check wall/CPU/RSS data, the 250 ms process timeline, host load, selected documents, corpus revision, and success/failure notes.
