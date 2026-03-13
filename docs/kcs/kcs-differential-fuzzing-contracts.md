# KCS: differential fuzzing and bad-input generation contracts

## Symptom

Fuzz campaign produces false positives (noise) for bad inputs, masks real crashes, or fails to exercise error-recovery paths in the parser/VM.

## Operational context

The differential fuzzer generates random Tcl scripts and runs them through multiple backends (VM unoptimised, VM optimised, C tclsh).  Mismatches between backends reveal correctness bugs.  A configurable fraction of inputs (`bad_input_pct`, default 5 %) are intentionally corrupted to exercise error-handling and recovery paths.  A coverage-guided (AFL-style) mutation loop uses `sys.settrace` to discover new code paths and grow the corpus.

## Decision rules / contracts

1. **Generator validity**: `TclGenerator.generate()` produces structurally valid Tcl by default.  The `bad_input_pct` knob is the *only* path that introduces intentionally malformed scripts.
2. **Corruption strategies**: `corrupt_script()` in `tcl_gen.py` applies exactly one corruption per call.  Strategies include: unbalanced braces, unbalanced quotes, truncation, garbage/control-char injection, null bytes, backslash-eol breakage, delimiter doubling, and broken-fragment replacement.
3. **Bad-input detection heuristic**: a script is marked `bad_input=True` when any of these hold: unbalanced `{}`, odd `"` count, unbalanced `[]`, or embedded null byte.  This heuristic is applied in `runner.py` and `coverage_guide.py` after generation/mutation.
4. **Oracle relaxation for bad inputs**: `FuzzResult.ok` returns `True` for bad inputs unless a *crash* (Python exception, `return_code == 2`) occurs.  Error-status and output mismatches between backends are expected for malformed input and are suppressed.
5. **Crash = always a finding**: a `return_code == 2` (unhandled exception) in any backend is always flagged, regardless of `bad_input` status.  Crashes indicate defensive-code gaps, not expected error handling.
6. **Mutation integration**: `mutate_script()` in `coverage_guide.py` also honours `bad_input_pct` — with that probability it delegates to `corrupt_script()` instead of the normal validity-preserving mutation strategies.
7. **Reproducibility**: every generated script is seeded.  `GenConfig.bad_input_pct` is propagated through `TclGenerator`, `run_campaign()`, and `run_coverage_guided()` so that campaigns are reproducible from `(base_seed, config)`.
8. **Stats tracking**: `CampaignStats.bad_inputs` counts how many scripts were detected as bad.  This is surfaced in CLI output and pytest campaign reports.

## File-path anchors

- `tests/fuzz/tcl_gen.py` — generator and corruption strategies (`corrupt_script`, `_corrupt_*`)
- `tests/fuzz/harness.py` — `FuzzResult.bad_input`, `run_differential(bad_input=…)`, oracle logic
- `tests/fuzz/coverage_guide.py` — `mutate_script(bad_input_pct=…)`, coverage-guided loop
- `tests/fuzz/runner.py` — `CampaignStats.bad_inputs`, bad-input detection heuristic, `--bad-input-pct` CLI flag
- `tests/fuzz/corpus/` — hand-written seed corpus (always valid)
- `tests/fuzz/findings/` — saved failing scripts

## Failure modes

- **False negatives from bad-input suppression**: if a real correctness bug only manifests on scripts that happen to trip the bad-input heuristic (e.g. the generator's pre-existing elseif brace imbalance), the mismatch is silently suppressed.  Mitigation: `bad_input` suppresses only `error_status`/`output` mismatches, never crashes.
- **Corruption not detected by heuristic**: some corruptions (e.g. backslash-eol) may not trip the delimiter-balance heuristic, so the script is not marked `bad_input`.  This means it is compared with the strict oracle — which may flag a mismatch that is really an expected error divergence.
- **Corpus pollution**: if a corrupted script discovers new coverage, it is added to the corpus in the coverage-guided loop.  Subsequent mutations of that script may produce many bad inputs, reducing effective coverage exploration.  Mitigation: keep `bad_input_pct` low (default 5 %).

## Test anchors

- `tests/test_fuzz_differential.py::TestGenerator::test_bad_input_pct_produces_some_bad`
- `tests/test_fuzz_differential.py::TestGenerator::test_bad_input_pct_zero_no_corruption`
- `tests/test_fuzz_differential.py::TestFuzzCampaign`
- `tests/test_fuzz_differential.py::TestOptimiserEquivalence`

## Discoverability

- [KCS index](README.md)
- [VM/bytecode test boundary](kcs-vm-bytecode-test-boundary.md)
