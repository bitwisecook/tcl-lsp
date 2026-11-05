# naming.error.c84-publication-order

Kind: `native-observation`

## Problem statement

An error without an explicit code can publish fresh NONE or accidentally reuse a prior global. A write trace also observes whether errorInfo or errorCode is assigned first and which setter spelling is used.

## Question

What result, error global header and callback order does the original empty-info error-code probe retain on each captured C release?

## Conclusion

The Tcl 8.4 controls preserve separate error-code-set state and the default versus supplied setter spelling. The actual five-release logs retain primary/header/reference observations at the probe frontier. The first probe and its separate digest remain separate evidence; outputs do not grant a modern return-options API on Tcl 8.4.

## Scope

Only the original retained programs, capture windows and provider labels in the attached artifacts are covered. Version labels are original receipt metadata, not a fresh patchlevel query. Missing command/API doors and aborted processes stay distinct from successful guest completion. Source files/probes explain the observer protocol; no pinned engine implementation excerpt or executed Rust assertion is claimed. Different attempts and sources are retained separately, never treated as byte-identical duplicates.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source. Version: tcl8.4 8.4.20.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`attempt1-manifest.json` (`file-5-row-6`):

```json
{
  "version": "8.4.20",
  "compile_exit": 0,
  "compile_stderr": "",
  "exit": 0,
  "stderr": ""
}
```

`manifest.json` (`file-13-row-14`):

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
  "original_table": "trace|conflict|::errorInfo|w|none|1|2|636f6e666c696374696e672076657273696f6e732070726f766964656420666f72207061636b616765202270726f6265223a20312c207468656e2032\ntrace|conflict|::errorCode|w|none|1|2|4e4f4e45\ntrace|conflict|::errorInfo|w|string|1|1|636f6e666c696374696e672076657273696f6e732070726f766964656420666f72207061636b616765202270726f6265223a20312c207468656e20320a202020207768696c6520657865637574696e670a227061636b6167652070726f766964652070726f6265203222\ntrace|conflict|::errorCode|r|none|1|1|4e4f4e45\nresult|conflict|0|none|1|1|7b636f6e666c696374696e672076657273696f6e732070726f766964656420666f72207061636b616765202270726f6265223a20312c207468656e20327d204e4f4e45\ntrace|no-code|::errorInfo|w|none|1|5|4641494c\ntrace|no-code|::errorCode|w|none|1|2|4e4f4e45\ntrace|no-code|::errorInfo|w|string|1|1|4641494c0a202020207768696c6520657865637574696e670a226572726f72204641494c22\ntrace|no-code|::errorCode|r|none|1|1|4e4f4e45\nresult|no-code|0|none|1|1|4641494c204e4f4e45\ntrace|NONE|errorCode|w|none|1|4|4e4f4e45\ntrace|NONE|::errorInfo|w|none|1|5|4641494c\ntrace|NONE|::errorInfo|w|string|1|1|4641494c0a202020207768696c6520657865637574696e670a226572726f72204641494c207b7d204e4f4e4522\ntrace|NONE|::errorCode|r|none|1|1|4e4f4e45\nresult|NONE|0|none|1|1|4641494c204e4f4e45\ntrace|structured|errorCode|w|none|1|4|435553544f4d2044455441494c\ntrace|structured|::errorInfo|w|none|1|5|4641494c\ntrace|structured|::errorInfo|w|string|1|1|4641494c0a202020207768696c6520657865637574696e670a226572726f72204641494c207b7d207b435553544f4d2044455441494c7d22\ntrace|structured|::errorCode|r|none|1|1|435553544f4d2044455441494c\nresult|structured|0|none|1|1|4641494c207b435553544f4d2044455441494c7d\ntrace|structured-getter|::errorInfo|w|string|1|2|62616420696e6465782022424144223a206d75737420626520696e7465676572206f7220656e643f2d696e74656765723f\ntrace|structured-getter|::errorCode|w|none|1|2|4e4f4e45\ntrace|structured-getter|::errorInfo|w|string|1|1|62616420696e6465782022424144223a206d75737420626520696e7465676572206f7220656e643f2d696e74656765723f0a202020207768696c6520657865637574696e670a226c696e646578207b7d2042414422\ntrace|structured-getter|::errorCode|r|none|1|1|4e4f4e45\nresult|structured-getter|0|none|1|1|7b62616420696e6465782022424144223a206d75737420626520696e7465676572206f7220656e643f2d696e74656765723f7d204e4f4e45\n"
}
```

### tcl8.5

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source. Version: tcl8.5 8.5.19.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`attempt1-manifest.json` (`file-5-row-7`):

```json
{
  "version": "8.5.19",
  "compile_exit": 0,
  "compile_stderr": "",
  "exit": 0,
  "stderr": ""
}
```

`manifest.json` (`file-13-row-15`):

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
  "original_table": "trace|conflict|::errorCode|w|list|0|3|4e4f4e45\ntrace|conflict|::errorInfo|w|string|1|2|636f6e666c696374696e672076657273696f6e732070726f766964656420666f72207061636b616765202270726f6265223a20312c207468656e20320a202020207768696c6520657865637574696e670a227061636b6167652070726f766964652070726f6265203222\ntrace|conflict|::errorCode|r|list|1|1|4e4f4e45\nresult|conflict|0|none|1|1|7b636f6e666c696374696e672076657273696f6e732070726f766964656420666f72207061636b616765202270726f6265223a20312c207468656e20327d204e4f4e45\ntrace|no-code|::errorCode|w|list|0|3|4e4f4e45\ntrace|no-code|::errorInfo|w|string|1|2|4641494c0a202020207768696c6520657865637574696e670a226572726f72204641494c22\ntrace|no-code|::errorCode|r|list|1|1|4e4f4e45\nresult|no-code|0|none|1|1|4641494c204e4f4e45\ntrace|NONE|::errorCode|w|list|1|7|4e4f4e45\ntrace|NONE|::errorInfo|w|string|1|2|4641494c0a202020207768696c6520657865637574696e670a226572726f72204641494c207b7d204e4f4e4522\ntrace|NONE|::errorCode|r|list|1|1|4e4f4e45\nresult|NONE|0|none|1|1|4641494c204e4f4e45\ntrace|structured|::errorCode|w|list|1|7|435553544f4d2044455441494c\ntrace|structured|::errorInfo|w|string|1|2|4641494c0a202020207768696c6520657865637574696e670a226572726f72204641494c207b7d207b435553544f4d2044455441494c7d22\ntrace|structured|::errorCode|r|list|1|1|435553544f4d2044455441494c\nresult|structured|0|none|1|1|4641494c207b435553544f4d2044455441494c7d\ntrace|structured-getter|::errorCode|w|list|0|3|4e4f4e45\ntrace|structured-getter|::errorInfo|w|string|1|2|62616420696e6465782022424144223a206d75737420626520696e74656765723f5b2b2d5d696e74656765723f206f7220656e643f5b2b2d5d696e74656765723f0a202020207768696c6520657865637574696e670a226c696e646578207b7d2042414422\ntrace|structured-getter|::errorCode|r|list|1|1|4e4f4e45\nresult|structured-getter|0|none|1|1|7b62616420696e6465782022424144223a206d75737420626520696e74656765723f5b2b2d5d696e74656765723f206f7220656e643f5b2b2d5d696e74656765723f7d204e4f4e45\n"
}
```

### tcl8.6

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source. Version: tcl8.6 8.6.18.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`attempt1-manifest.json` (`file-5-row-8`):

```json
{
  "version": "8.6.18",
  "compile_exit": 0,
  "compile_stderr": "",
  "exit": 0,
  "stderr": ""
}
```

`manifest.json` (`file-13-row-16`):

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
  "original_table": "trace|conflict|::errorCode|w|list|0|3|54434c205041434b4147452056455253494f4e434f4e464c494354\ntrace|conflict|::errorInfo|w|string|1|2|636f6e666c696374696e672076657273696f6e732070726f766964656420666f72207061636b616765202270726f6265223a20312c207468656e20320a202020207768696c6520657865637574696e670a227061636b6167652070726f766964652070726f6265203222\ntrace|conflict|::errorCode|r|list|1|1|54434c205041434b4147452056455253494f4e434f4e464c494354\nresult|conflict|0|none|1|1|7b636f6e666c696374696e672076657273696f6e732070726f766964656420666f72207061636b616765202270726f6265223a20312c207468656e20327d207b54434c205041434b4147452056455253494f4e434f4e464c4943547d\ntrace|no-code|::errorCode|w|list|0|3|4e4f4e45\ntrace|no-code|::errorInfo|w|string|1|2|4641494c0a202020207768696c6520657865637574696e670a226572726f72204641494c22\ntrace|no-code|::errorCode|r|list|1|1|4e4f4e45\nresult|no-code|0|none|1|1|4641494c204e4f4e45\ntrace|NONE|::errorCode|w|none|1|6|4e4f4e45\ntrace|NONE|::errorInfo|w|string|1|2|4641494c0a202020207768696c6520657865637574696e670a226572726f72204641494c207b7d204e4f4e4522\ntrace|NONE|::errorCode|r|none|1|2|4e4f4e45\nresult|NONE|0|none|1|1|4641494c204e4f4e45\ntrace|structured|::errorCode|w|none|1|6|435553544f4d2044455441494c\ntrace|structured|::errorInfo|w|string|1|2|4641494c0a202020207768696c6520657865637574696e670a226572726f72204641494c207b7d207b435553544f4d2044455441494c7d22\ntrace|structured|::errorCode|r|none|1|2|435553544f4d2044455441494c\nresult|structured|0|none|1|1|4641494c207b435553544f4d2044455441494c7d\ntrace|structured-getter|::errorCode|w|list|0|3|54434c2056414c554520494e444558\ntrace|structured-getter|::errorInfo|w|string|1|2|62616420696e6465782022424144223a206d75737420626520696e74656765723f5b2b2d5d696e74656765723f206f7220656e643f5b2b2d5d696e74656765723f0a202020207768696c6520657865637574696e670a226c696e646578207b7d2042414422\ntrace|structured-getter|::errorCode|r|list|1|1|54434c2056414c554520494e444558\nresult|structured-getter|0|none|1|1|7b62616420696e6465782022424144223a206d75737420626520696e74656765723f5b2b2d5d696e74656765723f206f7220656e643f5b2b2d5d696e74656765723f7d207b54434c2056414c554520494e4445587d\n"
}
```

### tcl9.0

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source. Version: tcl9.0 9.0.4.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`attempt1-manifest.json` (`file-5-row-9`):

```json
{
  "version": "9.0.4",
  "compile_exit": 0,
  "compile_stderr": "",
  "exit": 2,
  "stderr": ""
}
```

`manifest.json` (`file-13-row-17`):

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
  "original_table": "trace|conflict|::errorCode|write|list|0|3|54434c205041434b4147452056455253494f4e434f4e464c494354\ntrace|conflict|::errorInfo|write|string|1|2|636f6e666c696374696e672076657273696f6e732070726f766964656420666f72207061636b616765202270726f6265223a20312c207468656e20320a202020207768696c6520657865637574696e670a227061636b6167652070726f766964652070726f6265203222\ntrace|conflict|::errorCode|read|list|1|1|54434c205041434b4147452056455253494f4e434f4e464c494354\nresult|conflict|0|list|0|1|7b636f6e666c696374696e672076657273696f6e732070726f766964656420666f72207061636b616765202270726f6265223a20312c207468656e20327d207b54434c205041434b4147452056455253494f4e434f4e464c4943547d\ntrace|no-code|::errorCode|write|list|0|3|4e4f4e45\ntrace|no-code|::errorInfo|write|string|1|2|4641494c0a202020207768696c6520657865637574696e670a226572726f72204641494c22\ntrace|no-code|::errorCode|read|list|1|1|4e4f4e45\nresult|no-code|0|list|0|1|4641494c204e4f4e45\ntrace|NONE|::errorCode|write|none|1|6|4e4f4e45\ntrace|NONE|::errorInfo|write|string|1|2|4641494c0a202020207768696c6520657865637574696e670a226572726f72204641494c207b7d204e4f4e4522\ntrace|NONE|::errorCode|read|none|1|2|4e4f4e45\nresult|NONE|0|list|0|1|4641494c204e4f4e45\ntrace|structured|::errorCode|write|none|1|6|435553544f4d2044455441494c\ntrace|structured|::errorInfo|write|string|1|2|4641494c0a202020207768696c6520657865637574696e670a226572726f72204641494c207b7d207b435553544f4d2044455441494c7d22\ntrace|structured|::errorCode|read|none|1|2|435553544f4d2044455441494c\nresult|structured|0|list|0|1|4641494c207b435553544f4d2044455441494c7d\ntrace|structured-getter|::errorCode|write|list|0|3|54434c2056414c554520494e444558\ntrace|structured-getter|::errorInfo|write|string|1|2|62616420696e6465782022424144223a206d75737420626520696e74656765723f5b2b2d5d696e74656765723f206f7220656e643f5b2b2d5d696e74656765723f0a202020207768696c6520657865637574696e670a226c696e646578207b7d2042414422\ntrace|structured-getter|::errorCode|read|list|1|1|54434c2056414c554520494e444558\nresult|structured-getter|0|list|0|1|7b62616420696e6465782022424144223a206d75737420626520696e74656765723f5b2b2d5d696e74656765723f206f7220656e643f5b2b2d5d696e74656765723f7d207b54434c2056414c554520494e4445587d\n"
}
```

### tcl9.1

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source. Version: tcl9.1 9.1.0.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`attempt1-manifest.json` (`file-5-row-10`):

```json
{
  "version": "9.1.0",
  "compile_exit": 0,
  "compile_stderr": "",
  "exit": 2,
  "stderr": ""
}
```

`manifest.json` (`file-13-row-18`):

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
  "original_table": "trace|conflict|::errorCode|write|list|0|3|54434c205041434b4147452056455253494f4e434f4e464c494354\ntrace|conflict|::errorInfo|write|string|1|2|636f6e666c696374696e672076657273696f6e732070726f766964656420666f72207061636b616765202270726f6265223a20312c207468656e20320a202020207768696c6520657865637574696e670a227061636b6167652070726f766964652070726f6265203222\ntrace|conflict|::errorCode|read|list|1|1|54434c205041434b4147452056455253494f4e434f4e464c494354\nresult|conflict|0|list|0|1|7b636f6e666c696374696e672076657273696f6e732070726f766964656420666f72207061636b616765202270726f6265223a20312c207468656e20327d207b54434c205041434b4147452056455253494f4e434f4e464c4943547d\ntrace|no-code|::errorCode|write|list|0|3|4e4f4e45\ntrace|no-code|::errorInfo|write|string|1|2|4641494c0a202020207768696c6520657865637574696e670a226572726f72204641494c22\ntrace|no-code|::errorCode|read|list|1|1|4e4f4e45\nresult|no-code|0|list|0|1|4641494c204e4f4e45\ntrace|NONE|::errorCode|write|none|1|6|4e4f4e45\ntrace|NONE|::errorInfo|write|string|1|2|4641494c0a202020207768696c6520657865637574696e670a226572726f72204641494c207b7d204e4f4e4522\ntrace|NONE|::errorCode|read|none|1|2|4e4f4e45\nresult|NONE|0|list|0|1|4641494c204e4f4e45\ntrace|structured|::errorCode|write|none|1|6|435553544f4d2044455441494c\ntrace|structured|::errorInfo|write|string|1|2|4641494c0a202020207768696c6520657865637574696e670a226572726f72204641494c207b7d207b435553544f4d2044455441494c7d22\ntrace|structured|::errorCode|read|none|1|2|435553544f4d2044455441494c\nresult|structured|0|list|0|1|4641494c207b435553544f4d2044455441494c7d\ntrace|structured-getter|::errorCode|write|list|0|3|54434c2056414c554520494e444558\ntrace|structured-getter|::errorInfo|write|string|1|2|62616420696e6465782022424144223a206d75737420626520696e74656765723f5b2b2d5d696e74656765723f206f7220656e643f5b2b2d5d696e74656765723f0a202020207768696c6520657865637574696e670a226c696e646578207b7d2042414422\ntrace|structured-getter|::errorCode|read|list|1|1|54434c2056414c554520494e444558\nresult|structured-getter|0|list|0|1|7b62616420696e6465782022424144223a206d75737420626520696e74656765723f5b2b2d5d696e74656765723f206f7220656e643f5b2b2d5d696e74656765723f7d207b54434c2056414c554520494e4445587d\n"
}
```

### jim, bigip

Status: `not-tested`.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Consumer bindings

- [runtime/rust/src/interp/native_append.rs](../../../../runtime/rust/src/interp/native_append.rs), `interp::native_append::original_c84_counted_append_tests::original_c84_counted_worker_checks_getter_and_primary_retirement_before_write` (linked): The C84 counted append worker checks original getter and primary-retirement stages before its write, preserving actual observed references and earliest full cause. This is an inspected Rust software control; it supplies no passed assertion, external Native callback chronology, role parity or new getter/worker authority.
- [runtime/rust/src/interp/native_append.rs](../../../../runtime/rust/src/interp/native_append.rs), `interp::native_append::original_c84_counted_append_tests::original_c84_error_caller_preserves_reached_seed_and_blocks_later_code_and_result` (linked): C84 error-command refusal preserves its reached seed and blocks later explicit errorCode/result setters. This is an inspected Rust software control; it supplies no passed assertion, external Native callback chronology, role parity or new getter/worker authority.
- [runtime/rust/src/interp/native_append.rs](../../../../runtime/rust/src/interp/native_append.rs), `interp::native_append::original_c84_pending_scalar_tests::original_c84_cow_publishes_pending_allocation_and_settles_only_the_same_live_role` (linked): Actual C84 COW publishes a pending allocation with no cell Native hold, and settles the true cell role only for the same live ticket/allocation after old release. This is an inspected Rust software control; it supplies no passed assertion, external Native callback chronology, role parity or new getter/worker authority.
- [runtime/rust/src/interp/native_append.rs](../../../../runtime/rust/src/interp/native_append.rs), `interp::native_append::original_c84_pending_scalar_tests::original_c84_missing_cell_adopts_counted_message_before_write_without_working_hold` (linked): The selected missing-cell C84 counted message is adopted before the write callback with no artificial working hold. This is an inspected Rust software control; it supplies no passed assertion, external Native callback chronology, role parity or new getter/worker authority.

## Evidence

- [runtime/rust/tests/data/native_c84_error_code](../../../../runtime/rust/tests/data/native_c84_error_code)
- [runtime/rust/src/interp/native_append.rs](../../../../runtime/rust/src/interp/native_append.rs)
