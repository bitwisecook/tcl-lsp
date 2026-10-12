# Which namespace supplies an implicit ensemble target when global and caller commands shadow its name?

[Exact input](input.tcl) · [Scoped records](results.json) · [Build provenance](../../build-provenance/README.md)

| C release | Recorded runtime outcome | Raw stdout / stderr |
| --- | --- | --- |
| 8.5.19 | TARGETS {ADD ::M x} {ADD ::M y} {GLOBAL z}; MISSING 1 {invalid command name "::M::remove"} | [stdout](../../8.5.19/ensemble-shadow.stdout) / [stderr](../../8.5.19/ensemble-shadow.stderr) |
| 8.6.18 | TARGETS {ADD ::M x} {ADD ::M y} {GLOBAL z}; MISSING 0 {GLOBAL_REMOVE q} | [stdout](../../8.6.18/ensemble-shadow.stdout) / [stderr](../../8.6.18/ensemble-shadow.stderr) |
| 9.0.4 | TARGETS {ADD ::M x} {ADD ::M y} {GLOBAL z}; MISSING 0 {GLOBAL_REMOVE q} | [stdout](../../9.0.4/ensemble-shadow.stdout) / [stderr](../../9.0.4/ensemble-shadow.stderr) |
| 9.1.0 | TARGETS {ADD ::M x} {ADD ::M y} {GLOBAL z}; MISSING 0 {GLOBAL_REMOVE q} | [stdout](../../9.1.0/ensemble-shadow.stdout) / [stderr](../../9.1.0/ensemble-shadow.stderr) |

[C execution records](../../../native-runner/ensemble-shadow-c-records.json)

C 8.5.19: [stdout](../../../native-runner/ensemble-shadow-c-8.5.19.stdout) / [stderr](../../../native-runner/ensemble-shadow-c-8.5.19.stderr)

C 8.6.18: [stdout](../../../native-runner/ensemble-shadow-c-8.6.18.stdout) / [stderr](../../../native-runner/ensemble-shadow-c-8.6.18.stderr)

C 9.0.4: [stdout](../../../native-runner/ensemble-shadow-c-9.0.4.stdout) / [stderr](../../../native-runner/ensemble-shadow-c-9.0.4.stderr)

C 9.1.0: [stdout](../../../native-runner/ensemble-shadow-c-9.1.0.stdout) / [stderr](../../../native-runner/ensemble-shadow-c-9.1.0.stderr)

These are frozen process observations, not a current engine or WASM conformance claim. Jim and F5 were not measured for this question. The runtime command omits --init; this question does not establish matching-library initialization.
