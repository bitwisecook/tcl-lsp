# naming.compiler.warm-limit-pass-currency

Kind: `native-observation`

## Problem statement

A warm procedure can retain compiled state while a later limit check affects execution. Reusing a cold-pass observation would confuse compilation with limit settlement.

## Question

What compile-pass and epoch/count window does the original warm-limit probe observe on its captured releases?

## Conclusion

Only the independently captured warm-limit windows are covered. The complete original compile/run status and counter rows are retained; cold compilation rows and unavailable older-release doors supply no additional warm-limit answer.

## Scope

Only the original retained programs, capture windows and provider labels in the attached artifacts are covered. Version labels are original receipt metadata, not a fresh patchlevel query. Missing command/API doors and aborted processes stay distinct from successful guest completion. Source files/probes explain the observer protocol; no pinned engine implementation excerpt or executed Rust assertion is claimed. Different attempts and sources are retained separately, never treated as byte-identical duplicates.

## Provider answers

### tcl8.4, tcl8.5, jim, bigip

Status: `not-tested`.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### tcl8.6

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source. Version: tcl8.6 8.6.18.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-3-row-4`):

```json
{
  "version": "8.6.18",
  "compile_status": 0,
  "compile_stderr_hex": "",
  "status": 0,
  "stdout_hex": "30093209320935093509310a",
  "stderr_hex": ""
}
```

`8.6.18.tsv` (`file-0`):

```json
{
  "original_table": "0\t2\t2\t5\t5\t1\n"
}
```

### tcl9.0

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source. Version: tcl9.0 9.0.4.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-3-row-5`):

```json
{
  "version": "9.0.4",
  "compile_status": 0,
  "compile_stderr_hex": "",
  "status": 0,
  "stdout_hex": "30093209320935093509310a",
  "stderr_hex": ""
}
```

`9.0.4.tsv` (`file-1`):

```json
{
  "original_table": "0\t2\t2\t5\t5\t1\n"
}
```

### tcl9.1

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source. Version: tcl9.1 9.1.0.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-3-row-6`):

```json
{
  "version": "9.1.0",
  "compile_status": 0,
  "compile_stderr_hex": "",
  "status": 0,
  "stdout_hex": "30093209320935093509310a",
  "stderr_hex": ""
}
```

`9.1.0.tsv` (`file-2`):

```json
{
  "original_table": "0\t2\t2\t5\t5\t1\n"
}
```

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data/native_compiler_pass/warm_limit/8.6.18.tsv](../../../../rust/tcl-registry/tests/data/native_compiler_pass/warm_limit/8.6.18.tsv)
- [rust/tcl-registry/tests/data/native_compiler_pass/warm_limit/9.0.4.tsv](../../../../rust/tcl-registry/tests/data/native_compiler_pass/warm_limit/9.0.4.tsv)
- [rust/tcl-registry/tests/data/native_compiler_pass/warm_limit/9.1.0.tsv](../../../../rust/tcl-registry/tests/data/native_compiler_pass/warm_limit/9.1.0.tsv)
- [rust/tcl-registry/tests/data/native_compiler_pass/warm_limit/manifest.json](../../../../rust/tcl-registry/tests/data/native_compiler_pass/warm_limit/manifest.json)
- [rust/tcl-registry/tests/data/native_compiler_pass/warm_limit/probe.c](../../../../rust/tcl-registry/tests/data/native_compiler_pass/warm_limit/probe.c)
