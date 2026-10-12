# Actual C8.6 naming suite measurements

Current evidence: [upstream-consistent real in-process harness](20261012T193343Z/README.md). Actual C completes eight unchanged suites with 1182 passes / 85 skips / 0 failures. Native completes proc and rename with 40 passes / 17 skips / 0 failures; five other files refuse and ooNext2 exits with a script error. Both real-init startup markers and the real Tcltest package are reached. Actual Native all.tcl orchestration remains blocked before tests by ParseFormatArgs.

[Frame-enabled diagnostic](20261012T192755Z/README.md) and [default-frame diagnostic](20261012T193042Z/README.md) remain retained, separately labelled. Their wrapper configuration is not used for the final conformance totals. No unchanged-source identity or incomplete file is presented as a pass.

Replay the bounded question into a fresh automatically timestamped directory with `python experiments/name-handling/results/runtime-c86-real-suites/capture.py`; the exact pinned runner must already exist. No build is performed.
