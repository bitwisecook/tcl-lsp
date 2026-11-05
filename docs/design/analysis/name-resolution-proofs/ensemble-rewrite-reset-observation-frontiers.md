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

### tcl8.4, jim, bigip

Status: `not-tested`.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### tcl8.5

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval, Tcl_EvalObjEx, Tcl_EvalObjv. Exact lengths, flags and input constructors remain in the source. Version: tcl8.5 8.5.19.

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

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval, Tcl_EvalObjEx, Tcl_EvalObjv. Exact lengths, flags and input constructors remain in the source. Version: tcl8.6 8.6.18.

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

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval, Tcl_EvalObjEx, Tcl_EvalObjv. Exact lengths, flags and input constructors remain in the source. Version: tcl9.0 9.0.4.

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

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval, Tcl_EvalObjEx, Tcl_EvalObjv. Exact lengths, flags and input constructors remain in the source. Version: tcl9.1 9.1.0.

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

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data/native_ensemble_rewrite/manifest.json](../../../../rust/tcl-registry/tests/data/native_ensemble_rewrite/manifest.json)
- [rust/tcl-registry/tests/data/native_ensemble_rewrite/probe.c](../../../../rust/tcl-registry/tests/data/native_ensemble_rewrite/probe.c)
- [rust/tcl-registry/tests/data/native_ensemble_rewrite/reset-8.5.19.tsv](../../../../rust/tcl-registry/tests/data/native_ensemble_rewrite/reset-8.5.19.tsv)
- [rust/tcl-registry/tests/data/native_ensemble_rewrite/reset-8.6.18.tsv](../../../../rust/tcl-registry/tests/data/native_ensemble_rewrite/reset-8.6.18.tsv)
- [rust/tcl-registry/tests/data/native_ensemble_rewrite/reset-9.0.4.tsv](../../../../rust/tcl-registry/tests/data/native_ensemble_rewrite/reset-9.0.4.tsv)
- [rust/tcl-registry/tests/data/native_ensemble_rewrite/reset-9.1.0.tsv](../../../../rust/tcl-registry/tests/data/native_ensemble_rewrite/reset-9.1.0.tsv)
