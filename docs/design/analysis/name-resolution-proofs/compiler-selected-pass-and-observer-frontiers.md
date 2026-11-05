# naming.compiler.selected-pass-and-observer-frontiers

Kind: `native-observation`

## Problem statement

A native compiler can reject a selected body, run more than once or evaluate an observer at compilation entry. Counting only the final bytecode would hide the compilation and callback sequence.

## Question

What compile-pass counts, completion and callback windows occur in the fixed root/child/limit/observer controls?

## Conclusion

The original C compilation tables retain the selected cases and callback counts separately. The cases manifest is input documentation, not a new native run. Old release limit availability and actual child initialization remain release-specific; nothing establishes a Jim compiler.

## Scope

Only the original retained programs, capture windows and provider labels in the attached artifacts are covered. Version labels are original receipt metadata, not a fresh patchlevel query. Missing command/API doors and aborted processes stay distinct from successful guest completion. Source files/probes explain the observer protocol; no pinned engine implementation excerpt or executed Rust assertion is claimed. Different attempts and sources are retained separately, never treated as byte-identical duplicates.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_EvalEx, Tcl_EvalObjv. Exact lengths, flags and input constructors remain in the source. Version: tcl8.4 8.4.20.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-14-row-15`):

```json
{
  "version": "8.4.20",
  "compile_status": 0,
  "status": 0,
  "stdout_hex": "300930093009310a310930093009310a320930093009310a330930093009310a340930093009310a350931093009310a360931093009310a370931093009310a",
  "stderr_hex": ""
}
```

`8.4.20.tsv` (`file-0`):

```json
{
  "original_table": "0\t0\t0\t1\n1\t0\t0\t1\n2\t0\t0\t1\n3\t0\t0\t1\n4\t0\t0\t1\n5\t1\t0\t1\n6\t1\t0\t1\n7\t1\t0\t1\n"
}
```

### tcl8.5

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_EvalEx, Tcl_EvalObjv. Exact lengths, flags and input constructors remain in the source. Version: tcl8.5 8.5.19.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-14-row-16`):

```json
{
  "version": "8.5.19",
  "compile_status": 0,
  "status": 0,
  "stdout_hex": "300930093009310a310930093009310a320930093009310a330930093009310a340930093009310a350931093009310a360931093009310a370931093009310a",
  "stderr_hex": ""
}
```

`8.5.19.tsv` (`file-1`):

```json
{
  "original_table": "0\t0\t0\t1\n1\t0\t0\t1\n2\t0\t0\t1\n3\t0\t0\t1\n4\t0\t0\t1\n5\t1\t0\t1\n6\t1\t0\t1\n7\t1\t0\t1\n"
}
```

### tcl8.6

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_EvalEx, Tcl_EvalObjv. Exact lengths, flags and input constructors remain in the source. Version: tcl8.6 8.6.18.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`callback-manifest.json` (`file-8-row-9`):

```json
{
  "version": "8.6.18",
  "compile_status": 0,
  "status": 0,
  "stdout_hex": "30093009310932093509360a31093009310931093309340a",
  "stderr_hex": ""
}
```

`manifest.json` (`file-14-row-17`):

```json
{
  "version": "8.6.18",
  "compile_status": 0,
  "status": 0,
  "stdout_hex": "300930093209360a310930093109340a320930093109340a330930093109340a340930093009310a350931093109340a360931093209360a370931093109340a",
  "stderr_hex": ""
}
```

`8.6.18.tsv` (`file-2`):

```json
{
  "original_table": "0\t0\t2\t6\n1\t0\t1\t4\n2\t0\t1\t4\n3\t0\t1\t4\n4\t0\t0\t1\n5\t1\t1\t4\n6\t1\t2\t6\n7\t1\t1\t4\n"
}
```

`callback-8.6.18.tsv` (`file-5`):

```json
{
  "original_table": "0\t0\t1\t2\t5\t6\n1\t0\t1\t1\t3\t4\n"
}
```

### tcl9.0

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_EvalEx, Tcl_EvalObjv. Exact lengths, flags and input constructors remain in the source. Version: tcl9.0 9.0.4.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`callback-manifest.json` (`file-8-row-10`):

```json
{
  "version": "9.0.4",
  "compile_status": 0,
  "status": 0,
  "stdout_hex": "30093009310932093509360a31093009310931093309340a",
  "stderr_hex": ""
}
```

`manifest.json` (`file-14-row-18`):

```json
{
  "version": "9.0.4",
  "compile_status": 0,
  "status": 0,
  "stdout_hex": "300930093209360a310930093109340a320930093109340a330930093109340a340930093009310a350931093109340a360931093209360a370931093109340a",
  "stderr_hex": ""
}
```

`9.0.4.tsv` (`file-3`):

```json
{
  "original_table": "0\t0\t2\t6\n1\t0\t1\t4\n2\t0\t1\t4\n3\t0\t1\t4\n4\t0\t0\t1\n5\t1\t1\t4\n6\t1\t2\t6\n7\t1\t1\t4\n"
}
```

`callback-9.0.4.tsv` (`file-6`):

```json
{
  "original_table": "0\t0\t1\t2\t5\t6\n1\t0\t1\t1\t3\t4\n"
}
```

### tcl9.1

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_EvalEx, Tcl_EvalObjv. Exact lengths, flags and input constructors remain in the source. Version: tcl9.1 9.1.0.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`callback-manifest.json` (`file-8-row-11`):

```json
{
  "version": "9.1.0",
  "compile_status": 0,
  "status": 0,
  "stdout_hex": "30093009310932093509360a31093009310931093309340a",
  "stderr_hex": ""
}
```

`manifest.json` (`file-14-row-19`):

```json
{
  "version": "9.1.0",
  "compile_status": 0,
  "status": 0,
  "stdout_hex": "300930093209360a310930093109340a320930093109340a330930093109340a340930093009310a350931093109340a360931093209360a370931093109340a",
  "stderr_hex": ""
}
```

`9.1.0.tsv` (`file-4`):

```json
{
  "original_table": "0\t0\t2\t6\n1\t0\t1\t4\n2\t0\t1\t4\n3\t0\t1\t4\n4\t0\t0\t1\n5\t1\t1\t4\n6\t1\t2\t6\n7\t1\t1\t4\n"
}
```

`callback-9.1.0.tsv` (`file-7`):

```json
{
  "original_table": "0\t0\t1\t2\t5\t6\n1\t0\t1\t1\t3\t4\n"
}
```

### jim, bigip

Status: `not-tested`.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data/native_compiler_pass](../../../../rust/tcl-registry/tests/data/native_compiler_pass)
