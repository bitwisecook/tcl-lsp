# naming.ensemble.rewrite-reset-observation-frontiers

Kind: `native-observation`

## Problem statement

An active ensemble rewrite can survive a name updater, internal invoke or failed lookup while ordinary entry clears it. Reusing one reset rule across releases or entry modes changes wrong-argument presentation.

## Question

When does the actual native rewrite field clear in lookup, inline bytecode, internal invocation and missing-command controls?

## Conclusion

Tcl 8.5 clears after successful ordinary lookup and retains the rewrite during updater/inline/missing windows. Tcl 8.6 and C9 clear before lookup and on bytecode entry; internal invocation retains it. No C rewrite-field question was executed on Tcl 8.4 or Jim.

## Scope

Only the original retained programs, capture windows and provider labels in the attached artifacts are covered. Version labels are original receipt metadata, not a fresh patchlevel query. Missing command/API doors and aborted processes stay distinct from successful guest completion. Source files/probes explain the observer protocol; no pinned engine implementation excerpt or executed Rust assertion is claimed. Different attempts and sources are retained separately, never treated as byte-identical duplicates.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: tcl8.4.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval, Tcl_EvalObjEx, Tcl_EvalObjv. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.5.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-0-row-1`):

```json
{
  "version": "8.5.19",
  "exit": 0,
  "stdout": "lookup\tbefore=1 lookup=1 after=0 code=0\t0\nbytecode\tbefore=1 lookup=-1 after=1 code=0\t0\ninvoke\tbefore=1 lookup=-1 after=1 code=0\t0\nmissing\tbefore=1 lookup=-1 after=1 code=1\t0\n",
  "stderr": ""
}
```

`reset-8.5.19.tsv` (`file-6`):

```json
{
  "original_table": "lookup\tbefore=1 lookup=1 after=0 code=0\t0\nbytecode\tbefore=1 lookup=-1 after=1 code=0\t0\ninvoke\tbefore=1 lookup=-1 after=1 code=0\t0\nmissing\tbefore=1 lookup=-1 after=1 code=1\t0\n"
}
```

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval, Tcl_EvalObjEx, Tcl_EvalObjv. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.6.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-0-row-2`):

```json
{
  "version": "8.6.18",
  "exit": 0,
  "stdout": "lookup\tbefore=1 lookup=0 after=0 code=0\t0\nbytecode\tbefore=1 lookup=-1 after=0 code=0\t0\ninvoke\tbefore=1 lookup=-1 after=1 code=0\t0\nmissing\tbefore=1 lookup=-1 after=0 code=1\t0\n",
  "stderr": ""
}
```

`reset-8.6.18.tsv` (`file-7`):

```json
{
  "original_table": "lookup\tbefore=1 lookup=0 after=0 code=0\t0\nbytecode\tbefore=1 lookup=-1 after=0 code=0\t0\ninvoke\tbefore=1 lookup=-1 after=1 code=0\t0\nmissing\tbefore=1 lookup=-1 after=0 code=1\t0\n"
}
```

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval, Tcl_EvalObjEx, Tcl_EvalObjv. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl9.0.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-0-row-3`):

```json
{
  "version": "9.0.4",
  "exit": 0,
  "stdout": "lookup\tbefore=1 lookup=0 after=0 code=0\t0\nbytecode\tbefore=1 lookup=-1 after=0 code=0\t0\ninvoke\tbefore=1 lookup=-1 after=1 code=0\t0\nmissing\tbefore=1 lookup=-1 after=0 code=1\t0\n",
  "stderr": ""
}
```

`reset-9.0.4.tsv` (`file-8`):

```json
{
  "original_table": "lookup\tbefore=1 lookup=0 after=0 code=0\t0\nbytecode\tbefore=1 lookup=-1 after=0 code=0\t0\ninvoke\tbefore=1 lookup=-1 after=1 code=0\t0\nmissing\tbefore=1 lookup=-1 after=0 code=1\t0\n"
}
```

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval, Tcl_EvalObjEx, Tcl_EvalObjv. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl9.1.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-0-row-4`):

```json
{
  "version": "9.1.0",
  "exit": 0,
  "stdout": "lookup\tbefore=1 lookup=0 after=0 code=0\t0\nbytecode\tbefore=1 lookup=-1 after=0 code=0\t0\ninvoke\tbefore=1 lookup=-1 after=1 code=0\t0\nmissing\tbefore=1 lookup=-1 after=0 code=1\t0\n",
  "stderr": ""
}
```

`reset-9.1.0.tsv` (`file-9`):

```json
{
  "original_table": "lookup\tbefore=1 lookup=0 after=0 code=0\t0\nbytecode\tbefore=1 lookup=-1 after=0 code=0\t0\ninvoke\tbefore=1 lookup=-1 after=1 code=0\t0\nmissing\tbefore=1 lookup=-1 after=0 code=1\t0\n"
}
```

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: jim.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: bigip.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Exact evidence

- `file-0` (observation): [rust/tcl-registry/tests/data/native_ensemble_rewrite/manifest.json](../../../../rust/tcl-registry/tests/data/native_ensemble_rewrite/manifest.json). SHA-256 `ba02bbf6c217149ec46e0a84d3073add6c5b673f532cc82e9f414c4c2ae901e4`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-0-row-1` (provider): [rust/tcl-registry/tests/data/native_ensemble_rewrite/manifest.json](../../../../rust/tcl-registry/tests/data/native_ensemble_rewrite/manifest.json). SHA-256 `ba02bbf6c217149ec46e0a84d3073add6c5b673f532cc82e9f414c4c2ae901e4`. JSON pointer `/0`. Original provider/capture association at its exact selected row.
- `file-0-row-2` (provider): [rust/tcl-registry/tests/data/native_ensemble_rewrite/manifest.json](../../../../rust/tcl-registry/tests/data/native_ensemble_rewrite/manifest.json). SHA-256 `ba02bbf6c217149ec46e0a84d3073add6c5b673f532cc82e9f414c4c2ae901e4`. JSON pointer `/1`. Original provider/capture association at its exact selected row.
- `file-0-row-3` (provider): [rust/tcl-registry/tests/data/native_ensemble_rewrite/manifest.json](../../../../rust/tcl-registry/tests/data/native_ensemble_rewrite/manifest.json). SHA-256 `ba02bbf6c217149ec46e0a84d3073add6c5b673f532cc82e9f414c4c2ae901e4`. JSON pointer `/2`. Original provider/capture association at its exact selected row.
- `file-0-row-4` (provider): [rust/tcl-registry/tests/data/native_ensemble_rewrite/manifest.json](../../../../rust/tcl-registry/tests/data/native_ensemble_rewrite/manifest.json). SHA-256 `ba02bbf6c217149ec46e0a84d3073add6c5b673f532cc82e9f414c4c2ae901e4`. JSON pointer `/3`. Original provider/capture association at its exact selected row.
- `file-5` (input): [rust/tcl-registry/tests/data/native_ensemble_rewrite/probe.c](../../../../rust/tcl-registry/tests/data/native_ensemble_rewrite/probe.c). SHA-256 `4d9756556ec31be9a8365aeca65fe5fa68cbe7492b301d0fbc1d474271e74733`. Exact retained input/program bytes; purpose is limited to this question.
- `file-6` (observation): [rust/tcl-registry/tests/data/native_ensemble_rewrite/reset-8.5.19.tsv](../../../../rust/tcl-registry/tests/data/native_ensemble_rewrite/reset-8.5.19.tsv). SHA-256 `15c7ced87bbb56b454d5594fc936e6ed594279763e9b94d0f06a53310e581ab4`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-7` (observation): [rust/tcl-registry/tests/data/native_ensemble_rewrite/reset-8.6.18.tsv](../../../../rust/tcl-registry/tests/data/native_ensemble_rewrite/reset-8.6.18.tsv). SHA-256 `5d2d99e13fa884abc65f9b59a5e56806aa0911bf6062076f5f3abefc013bc8ff`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-8` (observation): [rust/tcl-registry/tests/data/native_ensemble_rewrite/reset-9.0.4.tsv](../../../../rust/tcl-registry/tests/data/native_ensemble_rewrite/reset-9.0.4.tsv). SHA-256 `5d2d99e13fa884abc65f9b59a5e56806aa0911bf6062076f5f3abefc013bc8ff`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-9` (observation): [rust/tcl-registry/tests/data/native_ensemble_rewrite/reset-9.1.0.tsv](../../../../rust/tcl-registry/tests/data/native_ensemble_rewrite/reset-9.1.0.tsv). SHA-256 `5d2d99e13fa884abc65f9b59a5e56806aa0911bf6062076f5f3abefc013bc8ff`. Exact retained capture/provenance artifact; purpose is limited to this question.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The attached artifacts retain captured outputs, available original inputs and recorded compile/run arguments. A missing input or wrapper limits exact replay; no new native execution is claimed. A replay requires the corresponding source/library/build configuration and original evaluator flags/argv; vanished outside capture paths are provenance, not executable replay dependencies. Where only source hex is retained, preserve those bytes. The per-purpose README/probe supplies the protocol; no new accepted reconfirmation or Rust pass is asserted.
