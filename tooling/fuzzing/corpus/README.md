# Differential-fuzzer seed corpus

This directory is a **data corpus, not importable package code.** It holds
a small set of hand-curated Tcl scripts that exercise distinct language
areas (arithmetic, strings, lists, control flow, procs, switch, catch,
namespaces, expr edge cases, nested structures).

## Consumed by

`TestCorpus` in `tooling/fuzzing/tests/test_fuzz_differential.py` replays
each script through the differential harness (`run_differential`) so the
hand-picked seeds stay green alongside the randomly-generated campaign.

Unlike `../findings/` (which the fuzzer writes to automatically), these
seeds are curated by hand and change rarely.
