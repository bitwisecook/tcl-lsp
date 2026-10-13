# Mapped original braced-source compiler controls

The captured compiler run selects 18 controls and records 13 passes and five
failures. `compiler-source.json` identifies the exact frozen source inventory;
all 5,851 paths match after execution. `compiler-start.json`,
`compiler-result.json`, `compiler-binary.json` and `compiler.log` retain the
selection, outcome, compiled artifact identity and raw output. No compiled
binary is retained.

The genuine original oracle program still reports zero W210 warnings where
the first C8.4 assertion expects two. Eight other Tcltest controls pass,
including the reported valid programs, alias forms and genuine missing-read
controls. The mapped original-word carrier therefore does not establish
completion of #2286.

One new source-only test incorrectly requests an installed IR procedure from
an analysis that supplies only retained source declarations. Two existing
selected-formal tests pass detached original tokens to an API that requires
their retained source binding. The independent detached-token refusal control
passes. Corrected test setups must use the genuine retained binding and keep
their positive and negative assertions; this receipt records the original
failing setups and does not claim those corrections have passed.

The retained-descendant lookup control reports `Ok([])` where it requires
`Err(RetainedDescendant)`. Its preceding current-local and lazy-global
assertions pass. A separate source-routing diagnosis and refreshed capture
must establish any repair. The native lookup cursor and both signature-scope
controls pass in this run.

This receipt covers compiler source controls. It supplies no new interpreter
oracle observation, runtime entry, physical cell, successful read or edit
authority. Real C and Jim braced-value observations are retained separately in
`experiments/name-handling/braced-source-correspondence/results.json`.
