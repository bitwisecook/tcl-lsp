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

Status: `observed`. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred. Version: tcl8.4 tcl8.4.

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

Status: `observed`. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred. Version: tcl8.5 tcl8.5.

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

Status: `observed`. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred. Version: tcl8.6 tcl8.6.

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

Status: `observed`. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred. Version: tcl9.0 tcl9.0.

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

Status: `observed`. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred. Version: tcl9.1 tcl9.1.

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

Status: `observed`. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred.

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

Status: `not-tested`.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-compiler/tests/data/native_double_completion/jim.txt](../../../../rust/tcl-compiler/tests/data/native_double_completion/jim.txt)
- [rust/tcl-compiler/tests/data/native_double_completion/manifest.json](../../../../rust/tcl-compiler/tests/data/native_double_completion/manifest.json)
- [rust/tcl-compiler/tests/data/native_double_completion/source.tcl](../../../../rust/tcl-compiler/tests/data/native_double_completion/source.tcl)
- [rust/tcl-compiler/tests/data/native_double_completion/tcl8.4.txt](../../../../rust/tcl-compiler/tests/data/native_double_completion/tcl8.4.txt)
- [rust/tcl-compiler/tests/data/native_double_completion/tcl8.5.txt](../../../../rust/tcl-compiler/tests/data/native_double_completion/tcl8.5.txt)
- [rust/tcl-compiler/tests/data/native_double_completion/tcl8.6.txt](../../../../rust/tcl-compiler/tests/data/native_double_completion/tcl8.6.txt)
- [rust/tcl-compiler/tests/data/native_double_completion/tcl9.0.txt](../../../../rust/tcl-compiler/tests/data/native_double_completion/tcl9.0.txt)
- [rust/tcl-compiler/tests/data/native_double_completion/tcl9.1.txt](../../../../rust/tcl-compiler/tests/data/native_double_completion/tcl9.1.txt)
