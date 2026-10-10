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

Status: `observed`. Version: 8.4.20. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.4.

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

Status: `observed`. Version: 8.5.19. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.5.

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

Status: `observed`. Version: 8.6.18. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.6.

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

Status: `observed`. Version: 9.0.4. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl9.0.

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

Status: `observed`. Version: 9.1.0. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl9.1.

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

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: jim.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: bigip.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Exact evidence

- `file-0` (observation): [runtime/rust/tests/data/native_c84_error_code/8.4.20.txt](../../../../runtime/rust/tests/data/native_c84_error_code/8.4.20.txt). SHA-256 `aa1858477af822c964b1c4e558e3b4642709cf911981a8123074ea4a7de9df1c`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-1` (observation): [runtime/rust/tests/data/native_c84_error_code/8.5.19.txt](../../../../runtime/rust/tests/data/native_c84_error_code/8.5.19.txt). SHA-256 `d7689a56ca6a49cc8305892d2b4dd4f581992f3cb9aa90c91d385e04c0a73f3f`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-2` (observation): [runtime/rust/tests/data/native_c84_error_code/8.6.18.txt](../../../../runtime/rust/tests/data/native_c84_error_code/8.6.18.txt). SHA-256 `2ab23272f6c6c44027ce0559464a26ef71c314505a1b4ba5dd47554a20bd2c5e`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-3` (observation): [runtime/rust/tests/data/native_c84_error_code/9.0.4.txt](../../../../runtime/rust/tests/data/native_c84_error_code/9.0.4.txt). SHA-256 `03d220c3c803ea4e2b5459dd6bc0a854340ba330f81e618184ee887b5f25f380`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-4` (observation): [runtime/rust/tests/data/native_c84_error_code/9.1.0.txt](../../../../runtime/rust/tests/data/native_c84_error_code/9.1.0.txt). SHA-256 `03d220c3c803ea4e2b5459dd6bc0a854340ba330f81e618184ee887b5f25f380`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-5` (observation): [runtime/rust/tests/data/native_c84_error_code/attempt1-manifest.json](../../../../runtime/rust/tests/data/native_c84_error_code/attempt1-manifest.json). SHA-256 `5ef0ae4f100e6f0cf93cc37fafc52bb579dc8b3ce75685298a1260364e6e3376`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-5-row-6` (provider): [runtime/rust/tests/data/native_c84_error_code/attempt1-manifest.json](../../../../runtime/rust/tests/data/native_c84_error_code/attempt1-manifest.json). SHA-256 `5ef0ae4f100e6f0cf93cc37fafc52bb579dc8b3ce75685298a1260364e6e3376`. JSON pointer `/0`. Original provider/capture association at its exact selected row.
- `file-5-row-7` (provider): [runtime/rust/tests/data/native_c84_error_code/attempt1-manifest.json](../../../../runtime/rust/tests/data/native_c84_error_code/attempt1-manifest.json). SHA-256 `5ef0ae4f100e6f0cf93cc37fafc52bb579dc8b3ce75685298a1260364e6e3376`. JSON pointer `/1`. Original provider/capture association at its exact selected row.
- `file-5-row-8` (provider): [runtime/rust/tests/data/native_c84_error_code/attempt1-manifest.json](../../../../runtime/rust/tests/data/native_c84_error_code/attempt1-manifest.json). SHA-256 `5ef0ae4f100e6f0cf93cc37fafc52bb579dc8b3ce75685298a1260364e6e3376`. JSON pointer `/2`. Original provider/capture association at its exact selected row.
- `file-5-row-9` (provider): [runtime/rust/tests/data/native_c84_error_code/attempt1-manifest.json](../../../../runtime/rust/tests/data/native_c84_error_code/attempt1-manifest.json). SHA-256 `5ef0ae4f100e6f0cf93cc37fafc52bb579dc8b3ce75685298a1260364e6e3376`. JSON pointer `/3`. Original provider/capture association at its exact selected row.
- `file-5-row-10` (provider): [runtime/rust/tests/data/native_c84_error_code/attempt1-manifest.json](../../../../runtime/rust/tests/data/native_c84_error_code/attempt1-manifest.json). SHA-256 `5ef0ae4f100e6f0cf93cc37fafc52bb579dc8b3ce75685298a1260364e6e3376`. JSON pointer `/4`. Original provider/capture association at its exact selected row.
- `file-11` (input): [runtime/rust/tests/data/native_c84_error_code/attempt1-probe.c](../../../../runtime/rust/tests/data/native_c84_error_code/attempt1-probe.c). SHA-256 `fedc32763ffc438fab710c9576587ff8fa94c13750ba031e6b2d6d63d99ecff2`. Exact retained input/program bytes; purpose is limited to this question.
- `file-12` (observation): [runtime/rust/tests/data/native_c84_error_code/controls.tsv](../../../../runtime/rust/tests/data/native_c84_error_code/controls.tsv). SHA-256 `71bc5cbadc3f38f692b4371b2e39b87cdce5d271940529a6bdfab19e5ae44fdb`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-13` (observation): [runtime/rust/tests/data/native_c84_error_code/manifest.json](../../../../runtime/rust/tests/data/native_c84_error_code/manifest.json). SHA-256 `4b7036526bcacf93bf640a3263b492174cb06f8510650a47092ad86ab14570c1`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-13-row-14` (provider): [runtime/rust/tests/data/native_c84_error_code/manifest.json](../../../../runtime/rust/tests/data/native_c84_error_code/manifest.json). SHA-256 `4b7036526bcacf93bf640a3263b492174cb06f8510650a47092ad86ab14570c1`. JSON pointer `/0`. Original provider/capture association at its exact selected row.
- `file-13-row-15` (provider): [runtime/rust/tests/data/native_c84_error_code/manifest.json](../../../../runtime/rust/tests/data/native_c84_error_code/manifest.json). SHA-256 `4b7036526bcacf93bf640a3263b492174cb06f8510650a47092ad86ab14570c1`. JSON pointer `/1`. Original provider/capture association at its exact selected row.
- `file-13-row-16` (provider): [runtime/rust/tests/data/native_c84_error_code/manifest.json](../../../../runtime/rust/tests/data/native_c84_error_code/manifest.json). SHA-256 `4b7036526bcacf93bf640a3263b492174cb06f8510650a47092ad86ab14570c1`. JSON pointer `/2`. Original provider/capture association at its exact selected row.
- `file-13-row-17` (provider): [runtime/rust/tests/data/native_c84_error_code/manifest.json](../../../../runtime/rust/tests/data/native_c84_error_code/manifest.json). SHA-256 `4b7036526bcacf93bf640a3263b492174cb06f8510650a47092ad86ab14570c1`. JSON pointer `/3`. Original provider/capture association at its exact selected row.
- `file-13-row-18` (provider): [runtime/rust/tests/data/native_c84_error_code/manifest.json](../../../../runtime/rust/tests/data/native_c84_error_code/manifest.json). SHA-256 `4b7036526bcacf93bf640a3263b492174cb06f8510650a47092ad86ab14570c1`. JSON pointer `/4`. Original provider/capture association at its exact selected row.
- `file-19` (input): [runtime/rust/tests/data/native_c84_error_code/probe.c](../../../../runtime/rust/tests/data/native_c84_error_code/probe.c). SHA-256 `b3f3510f1d6b4f5c2dd0d277f85bea17be03d7effc4b092fed4437eae0056b97`. Exact retained input/program bytes; purpose is limited to this question.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The attached artifacts retain captured outputs, available original inputs and recorded compile/run arguments. A missing input or wrapper limits exact replay; no new native execution is claimed. A replay requires the corresponding source/library/build configuration and original evaluator flags/argv; vanished outside capture paths are provenance, not executable replay dependencies. Where only source hex is retained, preserve those bytes. The per-purpose README/probe supplies the protocol; no new accepted reconfirmation or Rust pass is asserted.
