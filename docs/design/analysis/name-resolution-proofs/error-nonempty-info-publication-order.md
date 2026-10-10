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

Status: `observed`. Version: 8.4.20. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.4.

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

Status: `observed`. Version: 8.5.19. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.5.

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

Status: `observed`. Version: 8.6.18. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.6.

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

Status: `observed`. Version: 9.0.4. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl9.0.

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

Status: `observed`. Version: 9.1.0. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl9.1.

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

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: jim.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: bigip.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Exact evidence

- `file-0` (observation): [runtime/rust/tests/data/native_c84_error_code/nonempty/8.4.20.txt](../../../../runtime/rust/tests/data/native_c84_error_code/nonempty/8.4.20.txt). SHA-256 `084c04a731d933996ecac18378e740a36fab5a04341df977012261f76da9f906`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-1` (observation): [runtime/rust/tests/data/native_c84_error_code/nonempty/8.5.19.txt](../../../../runtime/rust/tests/data/native_c84_error_code/nonempty/8.5.19.txt). SHA-256 `ca95e46fe054bb1828b9d8c38e9fb57b181358d8932d288415e6504c1d7f3210`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-2` (observation): [runtime/rust/tests/data/native_c84_error_code/nonempty/8.6.18.txt](../../../../runtime/rust/tests/data/native_c84_error_code/nonempty/8.6.18.txt). SHA-256 `a267d47b796d4dc8b0c0018c55b3388018348a0d8d55c303f8e2c045b29622a8`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-3` (observation): [runtime/rust/tests/data/native_c84_error_code/nonempty/9.0.4.txt](../../../../runtime/rust/tests/data/native_c84_error_code/nonempty/9.0.4.txt). SHA-256 `2eb4d9092ef15094e18f06d33b1fb3df18947af2b9803e2b688178fae7d450cb`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-4` (observation): [runtime/rust/tests/data/native_c84_error_code/nonempty/9.1.0.txt](../../../../runtime/rust/tests/data/native_c84_error_code/nonempty/9.1.0.txt). SHA-256 `2eb4d9092ef15094e18f06d33b1fb3df18947af2b9803e2b688178fae7d450cb`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-5` (observation): [runtime/rust/tests/data/native_c84_error_code/nonempty/controls.tsv](../../../../runtime/rust/tests/data/native_c84_error_code/nonempty/controls.tsv). SHA-256 `500051d6727e6fea8b2e683609ceadd92526e3947b907111a0395660c30a1d52`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-6` (observation): [runtime/rust/tests/data/native_c84_error_code/nonempty/manifest.json](../../../../runtime/rust/tests/data/native_c84_error_code/nonempty/manifest.json). SHA-256 `27e20a699c572a236e348f4a14dea997f6c60ecf20b48b197ba32b2a25fec959`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-6-row-7` (provider): [runtime/rust/tests/data/native_c84_error_code/nonempty/manifest.json](../../../../runtime/rust/tests/data/native_c84_error_code/nonempty/manifest.json). SHA-256 `27e20a699c572a236e348f4a14dea997f6c60ecf20b48b197ba32b2a25fec959`. JSON pointer `/0`. Original provider/capture association at its exact selected row.
- `file-6-row-8` (provider): [runtime/rust/tests/data/native_c84_error_code/nonempty/manifest.json](../../../../runtime/rust/tests/data/native_c84_error_code/nonempty/manifest.json). SHA-256 `27e20a699c572a236e348f4a14dea997f6c60ecf20b48b197ba32b2a25fec959`. JSON pointer `/1`. Original provider/capture association at its exact selected row.
- `file-6-row-9` (provider): [runtime/rust/tests/data/native_c84_error_code/nonempty/manifest.json](../../../../runtime/rust/tests/data/native_c84_error_code/nonempty/manifest.json). SHA-256 `27e20a699c572a236e348f4a14dea997f6c60ecf20b48b197ba32b2a25fec959`. JSON pointer `/2`. Original provider/capture association at its exact selected row.
- `file-6-row-10` (provider): [runtime/rust/tests/data/native_c84_error_code/nonempty/manifest.json](../../../../runtime/rust/tests/data/native_c84_error_code/nonempty/manifest.json). SHA-256 `27e20a699c572a236e348f4a14dea997f6c60ecf20b48b197ba32b2a25fec959`. JSON pointer `/3`. Original provider/capture association at its exact selected row.
- `file-6-row-11` (provider): [runtime/rust/tests/data/native_c84_error_code/nonempty/manifest.json](../../../../runtime/rust/tests/data/native_c84_error_code/nonempty/manifest.json). SHA-256 `27e20a699c572a236e348f4a14dea997f6c60ecf20b48b197ba32b2a25fec959`. JSON pointer `/4`. Original provider/capture association at its exact selected row.
- `file-12` (input): [runtime/rust/tests/data/native_c84_error_code/nonempty/probe.c](../../../../runtime/rust/tests/data/native_c84_error_code/nonempty/probe.c). SHA-256 `23c80028c690ec5d2e1520b6ab867d5dd9fe8978c9627d0b04e5ed0a7e6b6379`. Exact retained input/program bytes; purpose is limited to this question.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The attached artifacts retain captured outputs, available original inputs and recorded compile/run arguments. A missing input or wrapper limits exact replay; no new native execution is claimed. A replay requires the corresponding source/library/build configuration and original evaluator flags/argv; vanished outside capture paths are provenance, not executable replay dependencies. Where only source hex is retained, preserve those bytes. The per-purpose README/probe supplies the protocol; no new accepted reconfirmation or Rust pass is asserted.
