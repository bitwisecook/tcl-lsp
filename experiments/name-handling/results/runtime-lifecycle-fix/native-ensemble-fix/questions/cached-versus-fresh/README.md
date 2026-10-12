# Can a retained namespace object still reach a child after its parent is unpublished, while a newly constructed value misses?

[Exact input](input.tcl) · [Scoped records](results.json) · [Build provenance](../../build-provenance/README.md)

| C release | Recorded runtime outcome | Raw stdout / stderr |
| --- | --- | --- |
| 8.5.19 | OBS {{parent 1 1} {child 0 0}} | [stdout](../../8.5.19/cached-fresh-namespace-probe.stdout) / [stderr](../../8.5.19/cached-fresh-namespace-probe.stderr) |
| 8.6.18 | host refusal; question unmeasured | [stdout](../../8.6.18/cached-fresh-namespace-probe.stdout) / [stderr](../../8.6.18/cached-fresh-namespace-probe.stderr) |
| 9.0.4 | OBS {{parent 1 1} {child 1 0}} | [stdout](../../9.0.4/cached-fresh-namespace-probe.stdout) / [stderr](../../9.0.4/cached-fresh-namespace-probe.stderr) |
| 9.1.0 | OBS {{parent 1 1} {child 1 0}} | [stdout](../../9.1.0/cached-fresh-namespace-probe.stdout) / [stderr](../../9.1.0/cached-fresh-namespace-probe.stderr) |

C 8.5.19: [stdout](../../../c-8.5.19-cached-fresh.stdout) / [stderr](../../../c-8.5.19-cached-fresh.stderr)

C 8.6.18: [stdout](../../../c-8.6.18-cached-fresh.stdout) / [stderr](../../../c-8.6.18-cached-fresh.stderr)

C 9.0.4: [stdout](../../../c-9.0.4-cached-fresh.stdout) / [stderr](../../../c-9.0.4-cached-fresh.stderr)

C 9.1.0: [stdout](../../../c-9.1.0-cached-fresh.stdout) / [stderr](../../../c-9.1.0-cached-fresh.stderr)

These are frozen process observations, not a current engine or WASM conformance claim. Jim and F5 were not measured for this question. C raw files are retained without a separate original execution-record manifest here; their source association is documented in the enclosing lifecycle report. Real 8.5 initialization differs from C; a core-only unit pass does not close that gap.
