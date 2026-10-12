# Does mutually entangled namespace deletion leave the global namespace intact under the real init.tcl and tcltest.tcl?

[Exact input](input.tcl) · [Scoped records](results.json) · [Build provenance](../../build-provenance/README.md)

| C release | Recorded runtime outcome | Raw stdout / stderr |
| --- | --- | --- |
| 8.4.20 | host refusal; question unmeasured | [stdout](../../8.4.20/upstream-namespace-56.2.stdout) / [stderr](../../8.4.20/upstream-namespace-56.2.stderr) |
| 8.5.19 | ---- namespace-56.2 start; :	Total	1	Passed	1	Skipped	0	Failed	0 | [stdout](../../8.5.19/upstream-namespace-56.2.stdout) / [stderr](../../8.5.19/upstream-namespace-56.2.stderr) |
| 8.6.18 | host refusal; question unmeasured | [stdout](../../8.6.18/upstream-namespace-56.2.stdout) / [stderr](../../8.6.18/upstream-namespace-56.2.stderr) |
| 9.0.4 | host refusal; question unmeasured | [stdout](../../9.0.4/upstream-namespace-56.2.stdout) / [stderr](../../9.0.4/upstream-namespace-56.2.stderr) |
| 9.1.0 | host refusal; question unmeasured | [stdout](../../9.1.0/upstream-namespace-56.2.stdout) / [stderr](../../9.1.0/upstream-namespace-56.2.stderr) |

[C execution records](../../../original-oracle-records.json)

C 8.4.20: [stdout](../../../c-8.4.20-original.stdout) / [stderr](../../../c-8.4.20-original.stderr)

C 8.5.19: [stdout](../../../c-8.5.19-original.stdout) / [stderr](../../../c-8.5.19-original.stderr)

C 8.6.18: [stdout](../../../c-8.6.18-original.stdout) / [stderr](../../../c-8.6.18-original.stderr)

C 9.0.4: [stdout](../../../c-9.0.4-original.stdout) / [stderr](../../../c-9.0.4-original.stderr)

C 9.1.0: [stdout](../../../c-9.1.0-original.stdout) / [stderr](../../../c-9.1.0-original.stderr)

These are frozen process observations, not a current engine or WASM conformance claim. Jim and F5 were not measured for this question. Only the runtime 8.5 capture reached the test; the other four startup/entry refusals provide no test coverage.
