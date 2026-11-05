# naming.ensemble.configuration-compiler-epochs

Kind: `native-observation`

## Problem statement

Repeated configuration, unknown-handler updates and rejected options can all look like an ensemble update but need not perform the same compiler-affecting setter transaction.

## Question

Which completed ensemble configuration updates advance the actual compiler epoch or change the original info compiler attachment?

## Conclusion

Successful configuration runs the native setter transaction even for repeated values; unknown/prefix setters and failed option parsing have separately recorded effects. Tcl 8.4 records the missing ensemble door. Counters remain relative to the same interpreter baseline.

## Scope

Only the original retained programs, capture windows and provider labels in the attached artifacts are covered. Version labels are original receipt metadata, not a fresh patchlevel query. Missing command/API doors and aborted processes stay distinct from successful guest completion. Source files/probes explain the observer protocol; no pinned engine implementation excerpt or executed Rust assertion is claimed. Different attempts and sources are retained separately, never treated as byte-identical duplicates.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source. Version: tcl8.4 8.4.20.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-5-row-6`):

```json
{
  "version": "8.4.20",
  "compile_exit": 0,
  "exit": 0,
  "observations": [
    "event\tcode\tcompiler\tinfo_hook",
    "initial\t0\t0\t0"
  ],
  "stderr": ""
}
```

`8.4.20.tsv` (`file-0`):

```json
{
  "original_table": "event\tcode\tcompiler\tinfo_hook\ninitial\t0\t0\t0\n"
}
```

### tcl8.5

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source. Version: tcl8.5 8.5.19.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-5-row-7`):

```json
{
  "version": "8.5.19",
  "compile_exit": 0,
  "exit": 0,
  "observations": [
    "event\tcode\tcompiler\tinfo_hook",
    "initial\t0\t3\t1",
    "prefix_false\t0\t5\t1",
    "same_prefix\t0\t7\t1",
    "bad_option\t1\t7\t1",
    "prefix_true\t0\t9\t1"
  ],
  "stderr": ""
}
```

`8.5.19.tsv` (`file-1`):

```json
{
  "original_table": "event\tcode\tcompiler\tinfo_hook\ninitial\t0\t3\t1\nprefix_false\t0\t5\t1\nsame_prefix\t0\t7\t1\nbad_option\t1\t7\t1\nprefix_true\t0\t9\t1\n"
}
```

### tcl8.6

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source. Version: tcl8.6 8.6.18.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-5-row-8`):

```json
{
  "version": "8.6.18",
  "compile_exit": 0,
  "exit": 0,
  "observations": [
    "event\tcode\tcompiler\tinfo_hook",
    "initial\t0\t17\t1",
    "prefix_false\t0\t20\t1",
    "same_prefix\t0\t23\t1",
    "bad_option\t1\t23\t1",
    "prefix_true\t0\t26\t1"
  ],
  "stderr": ""
}
```

`8.6.18.tsv` (`file-2`):

```json
{
  "original_table": "event\tcode\tcompiler\tinfo_hook\ninitial\t0\t17\t1\nprefix_false\t0\t20\t1\nsame_prefix\t0\t23\t1\nbad_option\t1\t23\t1\nprefix_true\t0\t26\t1\n"
}
```

### tcl9.0

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source. Version: tcl9.0 9.0.4.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-5-row-9`):

```json
{
  "version": "9.0.4",
  "compile_exit": 0,
  "exit": 0,
  "observations": [
    "event\tcode\tcompiler\tinfo_hook",
    "initial\t0\t20\t1",
    "prefix_false\t0\t23\t1",
    "same_prefix\t0\t26\t1",
    "bad_option\t1\t26\t1",
    "prefix_true\t0\t29\t1"
  ],
  "stderr": ""
}
```

`9.0.4.tsv` (`file-3`):

```json
{
  "original_table": "event\tcode\tcompiler\tinfo_hook\ninitial\t0\t20\t1\nprefix_false\t0\t23\t1\nsame_prefix\t0\t26\t1\nbad_option\t1\t26\t1\nprefix_true\t0\t29\t1\n"
}
```

### tcl9.1

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source. Version: tcl9.1 9.1.0.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-5-row-10`):

```json
{
  "version": "9.1.0",
  "compile_exit": 0,
  "exit": 0,
  "observations": [
    "event\tcode\tcompiler\tinfo_hook",
    "initial\t0\t24\t1",
    "prefix_false\t0\t27\t1",
    "same_prefix\t0\t30\t1",
    "bad_option\t1\t30\t1",
    "prefix_true\t0\t33\t1"
  ],
  "stderr": ""
}
```

`9.1.0.tsv` (`file-4`):

```json
{
  "original_table": "event\tcode\tcompiler\tinfo_hook\ninitial\t0\t24\t1\nprefix_false\t0\t27\t1\nsame_prefix\t0\t30\t1\nbad_option\t1\t30\t1\nprefix_true\t0\t33\t1\n"
}
```

### jim, bigip

Status: `not-tested`.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [runtime/rust/tests/data/native_ensemble_epochs/8.4.20.tsv](../../../../runtime/rust/tests/data/native_ensemble_epochs/8.4.20.tsv)
- [runtime/rust/tests/data/native_ensemble_epochs/8.5.19.tsv](../../../../runtime/rust/tests/data/native_ensemble_epochs/8.5.19.tsv)
- [runtime/rust/tests/data/native_ensemble_epochs/8.6.18.tsv](../../../../runtime/rust/tests/data/native_ensemble_epochs/8.6.18.tsv)
- [runtime/rust/tests/data/native_ensemble_epochs/9.0.4.tsv](../../../../runtime/rust/tests/data/native_ensemble_epochs/9.0.4.tsv)
- [runtime/rust/tests/data/native_ensemble_epochs/9.1.0.tsv](../../../../runtime/rust/tests/data/native_ensemble_epochs/9.1.0.tsv)
- [runtime/rust/tests/data/native_ensemble_epochs/manifest.json](../../../../runtime/rust/tests/data/native_ensemble_epochs/manifest.json)
- [runtime/rust/tests/data/native_ensemble_epochs/probe.c](../../../../runtime/rust/tests/data/native_ensemble_epochs/probe.c)
- [runtime/rust/tests/data/native_ensemble_epochs/run.py](../../../../runtime/rust/tests/data/native_ensemble_epochs/run.py)
