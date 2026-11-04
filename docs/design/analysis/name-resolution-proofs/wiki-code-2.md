# naming.wiki.code-2

Kind: `native-observation`

## Problem statement

The Tcl Wiki idiom and its adversarial reduction exercise captured namespace does not capture a proc local. A static consumer must distinguish the measured runtime result from the independent proof of source, namespace/frame, binding and lifecycle needed for navigation, diagnostics, edits and optimisation. The exact original input and per-case Tcltest receipts are retained.

## Question

Does this retained program establish that captured namespace does not capture a proc local?

## Conclusion

8.4.20: wiki-code-2 PASSED: actual result matched retained -result namespace. 8.5.19: wiki-code-2 PASSED: actual result matched retained -result namespace. 8.6.18: wiki-code-2 PASSED: actual result matched retained -result namespace. 9.0.4: wiki-code-2 PASSED: actual result matched retained -result namespace. 9.1.0: wiki-code-2 PASSED: actual result matched retained -result namespace. These are finite native script observations. They do not issue compiler source, current binding, object/cell, frame, edit or optimisation authority; Jim, BIG-IP and Rust implementation execution are separate obligations.

## Scope

Only retained authored test cases wiki-code-2 in stock C Tcl 8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 on 2026-10-12. Each test has a fresh child with normal startup; real matching upstream Tcltest drives assertions. Wiki pages provide pattern provenance, not provider authority. A skipped capability/version constraint supplies no native answer. Historical supplied VM/runtime binaries are outside these native-provider answers.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Recorded real C executable SHA-256 04f39dc867adab5acc96738a8dc04e47a3674b6487e698a0f8bb690f9fe3dd85; upstream tag core-8-4-20. Build flags and source-build correspondence are not recorded.. Channel: Authored Tcl script in fresh child interpreter; matching real init.tcl and Tcltest 2.2.11 test harness; binary/startup hashes retained.. Dialect: Tcl.

wiki-code-2 PASSED: actual result matched retained -result namespace.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Recorded real C executable SHA-256 46548301a578572298155e130fe2a6b856fc82b7cdf96a66e54d3eb9c2329961; upstream tag core-8-5-19. Build flags and source-build correspondence are not recorded.. Channel: Authored Tcl script in fresh child interpreter; matching real init.tcl and Tcltest 2.3.8 test harness; binary/startup hashes retained.. Dialect: Tcl.

wiki-code-2 PASSED: actual result matched retained -result namespace.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Recorded real C executable SHA-256 b307c18ab4946e932f72918d4803f01a1cd6ce8c28e8dd412c94876537e1190c; upstream tag core-8-6-18. Build flags and source-build correspondence are not recorded.. Channel: Authored Tcl script in fresh child interpreter; matching real init.tcl and Tcltest 2.5.11 test harness; binary/startup hashes retained.. Dialect: Tcl.

wiki-code-2 PASSED: actual result matched retained -result namespace.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Recorded real C executable SHA-256 cfe2b23524ca46502d2199084f8c56d4acea13353bde0bd5634fdede8dc92965; upstream tag core-9-0-4. Build flags and source-build correspondence are not recorded.. Channel: Authored Tcl script in fresh child interpreter; matching real init.tcl and Tcltest 2.5.11 test harness; binary/startup hashes retained.. Dialect: Tcl.

wiki-code-2 PASSED: actual result matched retained -result namespace.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Recorded real C executable SHA-256 cad3069775a57324bd43e7ea5a0ff20f8990fb483e0d2b7ed177686ce4d17cc7; upstream tag core-9-1-0. Build flags and source-build correspondence are not recorded.. Channel: Authored Tcl script in fresh child interpreter; matching real init.tcl and Tcltest 2.6.0 test harness; binary/startup hashes retained.. Dialect: Tcl.

wiki-code-2 PASSED: actual result matched retained -result namespace.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

This authored Wiki reduction was not executed on this provider. No equivalent result is inferred from C.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

This authored Wiki reduction was not executed on this provider. No equivalent result is inferred from C.

## Historical references

- `input` (input): [experiments/name-handling/patterns/wiki-probes.tcl](../../../../experiments/name-handling/patterns/wiki-probes.tcl). Complete authored Tcltest input, including fresh-child isolation and exact result assertions; no replacement tcltest/init implementation.
- `capture-runner` (input): [rust/tcl-registry/tests/data/native_wiki_naming_patterns/capture-runner-original.txt](../../../../rust/tcl-registry/tests/data/native_wiki_naming_patterns/capture-runner-original.txt). Exact immutable bytes of the actually executed C capture runner, preserved separately from maintained tooling; moving this snapshot does not make it a replay entry point.
- `observation-join` (observation): [rust/tcl-registry/tests/data/native_wiki_naming_patterns/observations.json](../../../../rust/tcl-registry/tests/data/native_wiki_naming_patterns/observations.json). JSON pointer `/questions/4`. Derived exact input/test/result-expression and raw PASS/SKIP receipt join; no manufactured intermediate result values.
- `tcl8.4-receipt` (provider): [experiments/name-handling/patterns/wiki-results/provenance.json](../../../../experiments/name-handling/patterns/wiki-results/provenance.json). JSON pointer `/records/0`. Actual reported patchlevel, binary hash, loaded init/tcltest byte hashes, invocation, exit status and transcript hashes.
- `tcl8.4-case-0` (observation): [experiments/name-handling/patterns/wiki-results/tcl8.4.20.stdout](../../../../experiments/name-handling/patterns/wiki-results/tcl8.4.20.stdout). Lines 6–6. Exact matching upstream Tcltest PASS or constraint SKIP line; PASS establishes only equality to the input result expression.
- `tcl8.5-receipt` (provider): [experiments/name-handling/patterns/wiki-results/provenance.json](../../../../experiments/name-handling/patterns/wiki-results/provenance.json). JSON pointer `/records/1`. Actual reported patchlevel, binary hash, loaded init/tcltest byte hashes, invocation, exit status and transcript hashes.
- `tcl8.5-case-0` (observation): [experiments/name-handling/patterns/wiki-results/tcl8.5.19.stdout](../../../../experiments/name-handling/patterns/wiki-results/tcl8.5.19.stdout). Lines 6–6. Exact matching upstream Tcltest PASS or constraint SKIP line; PASS establishes only equality to the input result expression.
- `tcl8.6-receipt` (provider): [experiments/name-handling/patterns/wiki-results/provenance.json](../../../../experiments/name-handling/patterns/wiki-results/provenance.json). JSON pointer `/records/2`. Actual reported patchlevel, binary hash, loaded init/tcltest byte hashes, invocation, exit status and transcript hashes.
- `tcl8.6-case-0` (observation): [experiments/name-handling/patterns/wiki-results/tcl8.6.18.stdout](../../../../experiments/name-handling/patterns/wiki-results/tcl8.6.18.stdout). Lines 6–6. Exact matching upstream Tcltest PASS or constraint SKIP line; PASS establishes only equality to the input result expression.
- `tcl9.0-receipt` (provider): [experiments/name-handling/patterns/wiki-results/provenance.json](../../../../experiments/name-handling/patterns/wiki-results/provenance.json). JSON pointer `/records/3`. Actual reported patchlevel, binary hash, loaded init/tcltest byte hashes, invocation, exit status and transcript hashes.
- `tcl9.0-case-0` (observation): [experiments/name-handling/patterns/wiki-results/tcl9.0.4.stdout](../../../../experiments/name-handling/patterns/wiki-results/tcl9.0.4.stdout). Lines 6–6. Exact matching upstream Tcltest PASS or constraint SKIP line; PASS establishes only equality to the input result expression.
- `tcl9.1-receipt` (provider): [experiments/name-handling/patterns/wiki-results/provenance.json](../../../../experiments/name-handling/patterns/wiki-results/provenance.json). JSON pointer `/records/4`. Actual reported patchlevel, binary hash, loaded init/tcltest byte hashes, invocation, exit status and transcript hashes.
- `tcl9.1-case-0` (observation): [experiments/name-handling/patterns/wiki-results/tcl9.1.0.stdout](../../../../experiments/name-handling/patterns/wiki-results/tcl9.1.0.stdout). Lines 6–6. Exact matching upstream Tcltest PASS or constraint SKIP line; PASS establishes only equality to the input result expression.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "experiments/name-handling/patterns/run_wiki_probes.py",
  "--bin-dir",
  "/workspace/tcl-lsp/tmp/name-tools/bin",
  "--output",
  "/tmp/wiki-probes-new"
]
```

Requires all five real matching C executables from the canonical release manifest and a new/empty output directory. The runner discovers matching startup bytes through the actual interpreter, including ZIPFS. No Rust consumer coverage or current-source equivalence is certified by native PASS. The original Wiki page revision IDs were unavailable; displayed Updated timestamps and selected example hashes are retained separately in experiments/name-handling/patterns/wiki-sources.json.
