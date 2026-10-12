# Does the global set command survive mutually entangled child deletion?

[Exact input](input.tcl) · [Scoped records](results.json) · [Build provenance](../../build-provenance/README.md)

| C release | Recorded runtime outcome | Raw stdout / stderr |
| --- | --- | --- |
| 8.4.20 | 0 set | [stdout](../../8.4.20/deletion-global-survival-probe.stdout) / [stderr](../../8.4.20/deletion-global-survival-probe.stderr) |
| 8.5.19 | 0 set | [stdout](../../8.5.19/deletion-global-survival-probe.stdout) / [stderr](../../8.5.19/deletion-global-survival-probe.stderr) |
| 8.6.18 | host refusal; question unmeasured | [stdout](../../8.6.18/deletion-global-survival-probe.stdout) / [stderr](../../8.6.18/deletion-global-survival-probe.stderr) |
| 9.0.4 | 0 set | [stdout](../../9.0.4/deletion-global-survival-probe.stdout) / [stderr](../../9.0.4/deletion-global-survival-probe.stderr) |
| 9.1.0 | 0 set | [stdout](../../9.1.0/deletion-global-survival-probe.stdout) / [stderr](../../9.1.0/deletion-global-survival-probe.stderr) |

[C execution records](../../../global-survival-oracle-records.json)

C 8.4.20: [stdout](../../../c-8.4.20-global-survival.stdout) / [stderr](../../../c-8.4.20-global-survival.stderr)

C 8.5.19: [stdout](../../../c-8.5.19-global-survival.stdout) / [stderr](../../../c-8.5.19-global-survival.stderr)

C 8.6.18: [stdout](../../../c-8.6.18-global-survival.stdout) / [stderr](../../../c-8.6.18-global-survival.stderr)

C 9.0.4: [stdout](../../../c-9.0.4-global-survival.stdout) / [stderr](../../../c-9.0.4-global-survival.stderr)

C 9.1.0: [stdout](../../../c-9.1.0-global-survival.stdout) / [stderr](../../../c-9.1.0-global-survival.stderr)

These are frozen process observations, not a current engine or WASM conformance claim. Jim and F5 were not measured for this question.
