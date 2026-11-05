# naming.error.nonempty-info-publication-order

Kind: `native-observation`

## Problem statement

A nonempty error-info argument adds publication and append work before final error-code/message settlement. Treating it as the empty-info path can reorder callback-visible state.

## Question

What original global values, callbacks and result are observed for nonempty error info with absent, NONE and structured codes?

## Conclusion

The nonempty-info probe retains its distinct publication frontier and original five-release physical observations. Its callback sequence cannot be substituted by the empty-info program or inferred from only the final message.

## Scope

Only the original retained programs, capture windows and provider labels in the attached artifacts are covered. Version labels are original receipt metadata, not a fresh patchlevel query. Missing command/API doors and aborted processes stay distinct from successful guest completion. Source files/probes explain the observer protocol; no pinned engine implementation excerpt or executed Rust assertion is claimed. Different attempts and sources are retained separately, never treated as byte-identical duplicates.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source. Version: tcl8.4 8.4.20.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-6-row-7`):

```json
{
  "version": "8.4.20",
  "compile_exit": 0,
  "compile_stderr": "",
  "exit": 0,
  "stderr": ""
}
```

`8.4.20.txt` (`file-0`):

```json
{
  "original_table": "trace|info-no-code|::errorInfo|w|none|1|2|\ntrace|info-no-code|::errorCode|w|none|1|2|4e4f4e45\ntrace|info-no-code|::errorInfo|w|string|1|1|494e464f\ntrace|info-no-code|::errorInfo|r|string|1|1|494e464f\ntrace|info-no-code|::errorCode|r|none|1|1|4e4f4e45\nresult|info-no-code|0|none|1|1|4641494c20494e464f204e4f4e45\ntrace|info-NONE|::errorInfo|w|none|1|2|\ntrace|info-NONE|::errorCode|w|none|1|2|4e4f4e45\ntrace|info-NONE|::errorInfo|w|string|1|1|494e464f\ntrace|info-NONE|errorCode|w|none|1|4|4e4f4e45\ntrace|info-NONE|::errorInfo|r|string|1|1|494e464f\ntrace|info-NONE|::errorCode|r|none|1|1|4e4f4e45\nresult|info-NONE|0|none|1|1|4641494c20494e464f204e4f4e45\ntrace|info-structured|::errorInfo|w|none|1|2|\ntrace|info-structured|::errorCode|w|none|1|2|4e4f4e45\ntrace|info-structured|::errorInfo|w|string|1|1|494e464f\ntrace|info-structured|errorCode|w|none|1|4|435553544f4d2044455441494c\ntrace|info-structured|::errorInfo|r|string|1|1|494e464f\ntrace|info-structured|::errorCode|r|none|1|1|435553544f4d2044455441494c\nresult|info-structured|0|none|1|1|4641494c20494e464f207b435553544f4d2044455441494c7d\n"
}
```

### tcl8.5

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source. Version: tcl8.5 8.5.19.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-6-row-8`):

```json
{
  "version": "8.5.19",
  "compile_exit": 0,
  "compile_stderr": "",
  "exit": 0,
  "stderr": ""
}
```

`8.5.19.txt` (`file-1`):

```json
{
  "original_table": "trace|info-no-code|::errorInfo|w|none|1|6|494e464f\ntrace|info-no-code|::errorCode|w|list|0|2|4e4f4e45\ntrace|info-no-code|::errorInfo|r|none|1|1|494e464f\ntrace|info-no-code|::errorCode|r|list|1|1|4e4f4e45\nresult|info-no-code|0|none|1|1|4641494c20494e464f204e4f4e45\ntrace|info-NONE|::errorInfo|w|none|1|6|494e464f\ntrace|info-NONE|::errorCode|w|list|1|5|4e4f4e45\ntrace|info-NONE|::errorInfo|r|none|1|1|494e464f\ntrace|info-NONE|::errorCode|r|list|1|1|4e4f4e45\nresult|info-NONE|0|none|1|1|4641494c20494e464f204e4f4e45\ntrace|info-structured|::errorInfo|w|none|1|6|494e464f\ntrace|info-structured|::errorCode|w|list|1|5|435553544f4d2044455441494c\ntrace|info-structured|::errorInfo|r|none|1|1|494e464f\ntrace|info-structured|::errorCode|r|list|1|1|435553544f4d2044455441494c\nresult|info-structured|0|none|1|1|4641494c20494e464f207b435553544f4d2044455441494c7d\n"
}
```

### tcl8.6

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source. Version: tcl8.6 8.6.18.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-6-row-9`):

```json
{
  "version": "8.6.18",
  "compile_exit": 0,
  "compile_stderr": "",
  "exit": 0,
  "stderr": ""
}
```

`8.6.18.txt` (`file-2`):

```json
{
  "original_table": "trace|info-no-code|::errorInfo|w|none|1|6|494e464f\ntrace|info-no-code|::errorCode|w|list|0|2|4e4f4e45\ntrace|info-no-code|::errorInfo|r|none|1|1|494e464f\ntrace|info-no-code|::errorCode|r|list|1|1|4e4f4e45\nresult|info-no-code|0|none|1|1|4641494c20494e464f204e4f4e45\ntrace|info-NONE|::errorInfo|w|none|1|6|494e464f\ntrace|info-NONE|::errorCode|w|none|1|5|4e4f4e45\ntrace|info-NONE|::errorInfo|r|none|1|1|494e464f\ntrace|info-NONE|::errorCode|r|none|1|1|4e4f4e45\nresult|info-NONE|0|none|1|1|4641494c20494e464f204e4f4e45\ntrace|info-structured|::errorInfo|w|none|1|6|494e464f\ntrace|info-structured|::errorCode|w|none|1|5|435553544f4d2044455441494c\ntrace|info-structured|::errorInfo|r|none|1|1|494e464f\ntrace|info-structured|::errorCode|r|none|1|1|435553544f4d2044455441494c\nresult|info-structured|0|none|1|1|4641494c20494e464f207b435553544f4d2044455441494c7d\n"
}
```

### tcl9.0

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source. Version: tcl9.0 9.0.4.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-6-row-10`):

```json
{
  "version": "9.0.4",
  "compile_exit": 0,
  "compile_stderr": "",
  "exit": 0,
  "stderr": ""
}
```

`9.0.4.txt` (`file-3`):

```json
{
  "original_table": "trace|info-no-code|::errorInfo|write|none|1|6|494e464f\ntrace|info-no-code|::errorCode|write|list|0|2|4e4f4e45\ntrace|info-no-code|::errorInfo|read|none|1|1|494e464f\ntrace|info-no-code|::errorCode|read|list|1|1|4e4f4e45\nresult|info-no-code|0|list|0|1|4641494c20494e464f204e4f4e45\ntrace|info-NONE|::errorInfo|write|none|1|6|494e464f\ntrace|info-NONE|::errorCode|write|none|1|5|4e4f4e45\ntrace|info-NONE|::errorInfo|read|none|1|1|494e464f\ntrace|info-NONE|::errorCode|read|none|1|1|4e4f4e45\nresult|info-NONE|0|list|0|1|4641494c20494e464f204e4f4e45\ntrace|info-structured|::errorInfo|write|none|1|6|494e464f\ntrace|info-structured|::errorCode|write|none|1|5|435553544f4d2044455441494c\ntrace|info-structured|::errorInfo|read|none|1|1|494e464f\ntrace|info-structured|::errorCode|read|none|1|1|435553544f4d2044455441494c\nresult|info-structured|0|list|0|1|4641494c20494e464f207b435553544f4d2044455441494c7d\n"
}
```

### tcl9.1

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source. Version: tcl9.1 9.1.0.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-6-row-11`):

```json
{
  "version": "9.1.0",
  "compile_exit": 0,
  "compile_stderr": "",
  "exit": 0,
  "stderr": ""
}
```

`9.1.0.txt` (`file-4`):

```json
{
  "original_table": "trace|info-no-code|::errorInfo|write|none|1|6|494e464f\ntrace|info-no-code|::errorCode|write|list|0|2|4e4f4e45\ntrace|info-no-code|::errorInfo|read|none|1|1|494e464f\ntrace|info-no-code|::errorCode|read|list|1|1|4e4f4e45\nresult|info-no-code|0|list|0|1|4641494c20494e464f204e4f4e45\ntrace|info-NONE|::errorInfo|write|none|1|6|494e464f\ntrace|info-NONE|::errorCode|write|none|1|5|4e4f4e45\ntrace|info-NONE|::errorInfo|read|none|1|1|494e464f\ntrace|info-NONE|::errorCode|read|none|1|1|4e4f4e45\nresult|info-NONE|0|list|0|1|4641494c20494e464f204e4f4e45\ntrace|info-structured|::errorInfo|write|none|1|6|494e464f\ntrace|info-structured|::errorCode|write|none|1|5|435553544f4d2044455441494c\ntrace|info-structured|::errorInfo|read|none|1|1|494e464f\ntrace|info-structured|::errorCode|read|none|1|1|435553544f4d2044455441494c\nresult|info-structured|0|list|0|1|4641494c20494e464f207b435553544f4d2044455441494c7d\n"
}
```

### jim, bigip

Status: `not-tested`.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Consumer bindings

- [runtime/rust/src/interp/native_append.rs](../../../../runtime/rust/src/interp/native_append.rs), `interp::native_append::original_c84_counted_append_tests::original_c84_write_outcome_settles_flags_before_refusing_later_getters` (linked): Reached C84 write callbacks settle the selected real error flags before refusing subsequent getters; Host/context cause remains terminal. This is an inspected Rust software control; it supplies no passed assertion, external Native callback chronology, role parity or new getter/worker authority.

## Evidence

- [runtime/rust/tests/data/native_c84_error_code/nonempty](../../../../runtime/rust/tests/data/native_c84_error_code/nonempty)
- [runtime/rust/src/interp/native_append.rs](../../../../runtime/rust/src/interp/native_append.rs)
