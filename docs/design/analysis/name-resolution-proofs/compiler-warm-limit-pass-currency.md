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

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: tcl8.4.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: tcl8.5.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.6.

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

Status: `observed`. Version: 9.0.4. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl9.0.

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

Status: `observed`. Version: 9.1.0. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl9.1.

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

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: jim.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: bigip.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Exact evidence

- `file-0` (observation): [rust/tcl-registry/tests/data/native_compiler_pass/warm_limit/8.6.18.tsv](../../../../rust/tcl-registry/tests/data/native_compiler_pass/warm_limit/8.6.18.tsv). SHA-256 `0e91d07512f9770319781d7fe1d3c25f9bd80955459dd0a6b00bc2c1e5b7e774`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-1` (observation): [rust/tcl-registry/tests/data/native_compiler_pass/warm_limit/9.0.4.tsv](../../../../rust/tcl-registry/tests/data/native_compiler_pass/warm_limit/9.0.4.tsv). SHA-256 `0e91d07512f9770319781d7fe1d3c25f9bd80955459dd0a6b00bc2c1e5b7e774`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-2` (observation): [rust/tcl-registry/tests/data/native_compiler_pass/warm_limit/9.1.0.tsv](../../../../rust/tcl-registry/tests/data/native_compiler_pass/warm_limit/9.1.0.tsv). SHA-256 `0e91d07512f9770319781d7fe1d3c25f9bd80955459dd0a6b00bc2c1e5b7e774`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-3` (observation): [rust/tcl-registry/tests/data/native_compiler_pass/warm_limit/manifest.json](../../../../rust/tcl-registry/tests/data/native_compiler_pass/warm_limit/manifest.json). SHA-256 `42b9689024d35857f41d9977ce23b7f4371f577cd1063e417c37d54a19f6e1ec`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-3-row-4` (provider): [rust/tcl-registry/tests/data/native_compiler_pass/warm_limit/manifest.json](../../../../rust/tcl-registry/tests/data/native_compiler_pass/warm_limit/manifest.json). SHA-256 `42b9689024d35857f41d9977ce23b7f4371f577cd1063e417c37d54a19f6e1ec`. JSON pointer `/0`. Original provider/capture association at its exact selected row.
- `file-3-row-5` (provider): [rust/tcl-registry/tests/data/native_compiler_pass/warm_limit/manifest.json](../../../../rust/tcl-registry/tests/data/native_compiler_pass/warm_limit/manifest.json). SHA-256 `42b9689024d35857f41d9977ce23b7f4371f577cd1063e417c37d54a19f6e1ec`. JSON pointer `/1`. Original provider/capture association at its exact selected row.
- `file-3-row-6` (provider): [rust/tcl-registry/tests/data/native_compiler_pass/warm_limit/manifest.json](../../../../rust/tcl-registry/tests/data/native_compiler_pass/warm_limit/manifest.json). SHA-256 `42b9689024d35857f41d9977ce23b7f4371f577cd1063e417c37d54a19f6e1ec`. JSON pointer `/2`. Original provider/capture association at its exact selected row.
- `file-7` (input): [rust/tcl-registry/tests/data/native_compiler_pass/warm_limit/probe.c](../../../../rust/tcl-registry/tests/data/native_compiler_pass/warm_limit/probe.c). SHA-256 `e1d4472a49de8e9e29bbe507c2f49dd48ddf7815daf962a1e1cd7d30f18565d8`. Exact retained input/program bytes; purpose is limited to this question.
- `file-8` (input): [rust/tcl-registry/tests/data/native_compiler_pass/warm_limit/run.py](../../../../rust/tcl-registry/tests/data/native_compiler_pass/warm_limit/run.py). SHA-256 `8a78d146ebe442e0913a68b5636e9d9189fb5453cdf9732a945bb95855471140`. Exact retained input/program bytes; purpose is limited to this question.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The attached artifacts retain captured outputs, available original inputs and recorded compile/run arguments. A missing input or wrapper limits exact replay; no new native execution is claimed. A replay requires the corresponding source/library/build configuration and original evaluator flags/argv; vanished outside capture paths are provenance, not executable replay dependencies. Where only source hex is retained, preserve those bytes. The per-purpose README/probe supplies the protocol; no new accepted reconfirmation or Rust pass is asserted.
