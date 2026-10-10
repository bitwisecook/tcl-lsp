# naming.expression.double-completion-settlement

Kind: `native-observation`

## Problem statement

Numeric success, rejected infinity, a variable-read error and operand Return all reach a double conversion through different frontiers. Assuming conversion success proves the entire expression settled normally is unsafe.

## Question

What original numeric, argument and read/dispatch completions do the ten double-expression controls retain on each engine?

## Conclusion

Tcl 8.4 rejects the recorded infinity value while later C and Jim accept it. Original argument Return/Error and C read failures remain separate rows. Modern C command replacement and older fixed-table behavior are independent controls; Jim trace absence remains explicit.

## Scope

Only the original retained programs, capture windows and provider labels in the attached artifacts are covered. Version labels are original receipt metadata, not a fresh patchlevel query. Missing command/API doors and aborted processes stay distinct from successful guest completion. Source files/probes explain the observer protocol; no pinned engine implementation excerpt or executed Rust assertion is claimed. Different attempts and sources are retained separately, never treated as byte-identical duplicates.

## Provider answers

### tcl8.4

Status: `observed`. Version: tcl8.4. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred.. Dialect: tcl8.4.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-1-row-2`):

```json
{
  "profile": "tcl8.4",
  "exit": 0,
  "stderr": ""
}
```

`tcl8.4.txt` (`file-9`):

```json
{
  "original_table": "double21 0 21.0\ninvalid 1 {argument to math function didn't have numeric value}\ninfinity 1 {floating-point value too large to represent}\nnegative 0 -21.0\narity_zero 1 {too few arguments for math function}\narity_two 1 {too many arguments for math function}\nargument_return 0 EARLY\nargument_error 1 ARGUMENT\nread_error 1 {can't read \"original\": READ_ERROR}\nreplacement 0 21.0\n"
}
```

### tcl8.5

Status: `observed`. Version: tcl8.5. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred.. Dialect: tcl8.5.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-1-row-3`):

```json
{
  "profile": "tcl8.5",
  "exit": 0,
  "stderr": ""
}
```

`tcl8.5.txt` (`file-10`):

```json
{
  "original_table": "double21 0 21.0\ninvalid 1 {expected floating-point number but got \"invalid\"}\ninfinity 0 Inf\nnegative 0 -21.0\narity_zero 1 {too few arguments for math function \"double\"}\narity_two 1 {too many arguments for math function \"double\"}\nargument_return 0 EARLY\nargument_error 1 ARGUMENT\nread_error 1 {can't read \"original\": READ_ERROR}\nreplacement 1 REPLACED\n"
}
```

### tcl8.6

Status: `observed`. Version: tcl8.6. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred.. Dialect: tcl8.6.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-1-row-4`):

```json
{
  "profile": "tcl8.6",
  "exit": 0,
  "stderr": ""
}
```

`tcl8.6.txt` (`file-11`):

```json
{
  "original_table": "double21 0 21.0\ninvalid 1 {expected floating-point number but got \"invalid\"}\ninfinity 0 Inf\nnegative 0 -21.0\narity_zero 1 {not enough arguments for math function \"double\"}\narity_two 1 {too many arguments for math function \"double\"}\nargument_return 0 EARLY\nargument_error 1 ARGUMENT\nread_error 1 {can't read \"original\": READ_ERROR}\nreplacement 1 REPLACED\n"
}
```

### tcl9.0

Status: `observed`. Version: tcl9.0. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred.. Dialect: tcl9.0.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-1-row-5`):

```json
{
  "profile": "tcl9.0",
  "exit": 0,
  "stderr": ""
}
```

`tcl9.0.txt` (`file-12`):

```json
{
  "original_table": "double21 0 21.0\ninvalid 1 {expected floating-point number but got \"invalid\"}\ninfinity 0 Inf\nnegative 0 -21.0\narity_zero 1 {not enough arguments for math function \"double\"}\narity_two 1 {too many arguments for math function \"double\"}\nargument_return 0 EARLY\nargument_error 1 ARGUMENT\nread_error 1 {can't read \"original\": READ_ERROR}\nreplacement 1 REPLACED\n"
}
```

### tcl9.1

Status: `observed`. Version: tcl9.1. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred.. Dialect: tcl9.1.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-1-row-6`):

```json
{
  "profile": "tcl9.1",
  "exit": 0,
  "stderr": ""
}
```

`tcl9.1.txt` (`file-13`):

```json
{
  "original_table": "double21 0 21.0\ninvalid 1 {expected floating-point number but got \"invalid\"}\ninfinity 0 Inf\nnegative 0 -21.0\narity_zero 1 {not enough arguments for math function \"double\"}\narity_two 1 {too many arguments for math function \"double\"}\nargument_return 0 EARLY\nargument_error 1 ARGUMENT\nread_error 1 {can't read \"original\": READ_ERROR}\nreplacement 1 REPLACED\n"
}
```

### jim

Status: `observed`. Version: jim. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred.. Dialect: jim.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-1-row-7`):

```json
{
  "profile": "jim",
  "exit": 0,
  "stderr": ""
}
```

`jim.txt` (`file-0`):

```json
{
  "original_table": "double21 0 21.0\ninvalid 1 {expected boolean but got \"invalid\"}\ninfinity 0 Inf\nnegative 0 -21.0\narity_zero 1 {too few arguments for math function}\narity_two 1 {too many arguments for math function}\nargument_return 0 EARLY\nargument_error 1 ARGUMENT\nread_trace_unavailable 0 unavailable\nreplacement 0 21.0\n"
}
```

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: bigip.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Exact evidence

- `file-0` (observation): [rust/tcl-compiler/tests/data/native_double_completion/jim.txt](../../../../rust/tcl-compiler/tests/data/native_double_completion/jim.txt). SHA-256 `afe5d9bc1e36777a8463d6eec090579f10bbb6e7306fe84a3f23ea7f7a200b72`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-1` (observation): [rust/tcl-compiler/tests/data/native_double_completion/manifest.json](../../../../rust/tcl-compiler/tests/data/native_double_completion/manifest.json). SHA-256 `cb870998bb1f2ecad0c557134b7adad280b5417fe621343e19778459b5870e6c`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-1-row-2` (provider): [rust/tcl-compiler/tests/data/native_double_completion/manifest.json](../../../../rust/tcl-compiler/tests/data/native_double_completion/manifest.json). SHA-256 `cb870998bb1f2ecad0c557134b7adad280b5417fe621343e19778459b5870e6c`. JSON pointer `/native_runs/0`. Original provider/capture association at its exact selected row.
- `file-1-row-3` (provider): [rust/tcl-compiler/tests/data/native_double_completion/manifest.json](../../../../rust/tcl-compiler/tests/data/native_double_completion/manifest.json). SHA-256 `cb870998bb1f2ecad0c557134b7adad280b5417fe621343e19778459b5870e6c`. JSON pointer `/native_runs/1`. Original provider/capture association at its exact selected row.
- `file-1-row-4` (provider): [rust/tcl-compiler/tests/data/native_double_completion/manifest.json](../../../../rust/tcl-compiler/tests/data/native_double_completion/manifest.json). SHA-256 `cb870998bb1f2ecad0c557134b7adad280b5417fe621343e19778459b5870e6c`. JSON pointer `/native_runs/2`. Original provider/capture association at its exact selected row.
- `file-1-row-5` (provider): [rust/tcl-compiler/tests/data/native_double_completion/manifest.json](../../../../rust/tcl-compiler/tests/data/native_double_completion/manifest.json). SHA-256 `cb870998bb1f2ecad0c557134b7adad280b5417fe621343e19778459b5870e6c`. JSON pointer `/native_runs/3`. Original provider/capture association at its exact selected row.
- `file-1-row-6` (provider): [rust/tcl-compiler/tests/data/native_double_completion/manifest.json](../../../../rust/tcl-compiler/tests/data/native_double_completion/manifest.json). SHA-256 `cb870998bb1f2ecad0c557134b7adad280b5417fe621343e19778459b5870e6c`. JSON pointer `/native_runs/4`. Original provider/capture association at its exact selected row.
- `file-1-row-7` (provider): [rust/tcl-compiler/tests/data/native_double_completion/manifest.json](../../../../rust/tcl-compiler/tests/data/native_double_completion/manifest.json). SHA-256 `cb870998bb1f2ecad0c557134b7adad280b5417fe621343e19778459b5870e6c`. JSON pointer `/native_runs/5`. Original provider/capture association at its exact selected row.
- `file-8` (input): [rust/tcl-compiler/tests/data/native_double_completion/source.tcl](../../../../rust/tcl-compiler/tests/data/native_double_completion/source.tcl). SHA-256 `8b2d3190bc70c8df7c6c69852a81cf527599be6ce0bf5978b01ff5834c86fdbd`. Exact retained input/program bytes; purpose is limited to this question.
- `file-9` (observation): [rust/tcl-compiler/tests/data/native_double_completion/tcl8.4.txt](../../../../rust/tcl-compiler/tests/data/native_double_completion/tcl8.4.txt). SHA-256 `1be35194ef61529941438b3cf4e1c05458aef45de40ecead1e9c0bb0b1b0c2d9`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-10` (observation): [rust/tcl-compiler/tests/data/native_double_completion/tcl8.5.txt](../../../../rust/tcl-compiler/tests/data/native_double_completion/tcl8.5.txt). SHA-256 `3a22947c5cce36d2461d1ab5f122d11acaca94eb9f3800e11ebcab2a4fa92a96`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-11` (observation): [rust/tcl-compiler/tests/data/native_double_completion/tcl8.6.txt](../../../../rust/tcl-compiler/tests/data/native_double_completion/tcl8.6.txt). SHA-256 `868dd55d14bd5d56f0b2dadbe694f81bf7f2e1570b9ccec9496f55bd6c644058`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-12` (observation): [rust/tcl-compiler/tests/data/native_double_completion/tcl9.0.txt](../../../../rust/tcl-compiler/tests/data/native_double_completion/tcl9.0.txt). SHA-256 `868dd55d14bd5d56f0b2dadbe694f81bf7f2e1570b9ccec9496f55bd6c644058`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-13` (observation): [rust/tcl-compiler/tests/data/native_double_completion/tcl9.1.txt](../../../../rust/tcl-compiler/tests/data/native_double_completion/tcl9.1.txt). SHA-256 `868dd55d14bd5d56f0b2dadbe694f81bf7f2e1570b9ccec9496f55bd6c644058`. Exact retained capture/provenance artifact; purpose is limited to this question.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The attached artifacts retain captured outputs, available original inputs and recorded compile/run arguments. A missing input or wrapper limits exact replay; no new native execution is claimed. A replay requires the corresponding source/library/build configuration and original evaluator flags/argv; vanished outside capture paths are provenance, not executable replay dependencies. Where only source hex is retained, preserve those bytes. The per-purpose README/probe supplies the protocol; no new accepted reconfirmation or Rust pass is asserted.
