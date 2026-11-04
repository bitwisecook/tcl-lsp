# naming.native-abi.callback-completion-result-and-options

Kind: `native-observation`

## Problem statement

A C worker can return OK, Error, Return, Break, Continue or a custom code while keeping the same original result object. Converting all codes to a guest error would lose native completion and options identity.

## Question

Which original result and return-options object windows are observed for codes 0,1,2,3,4,7 and supplied versus absent error codes?

## Conclusion

The probe retains native completion separately from the original result and options. Tcl 8.4 lacks Tcl_GetReturnOptions, so its rows establish code/result only. The companion duplicate-child List probe separately retains alias identities and C9 errorInfo ownership; older C string-copy results cannot be promoted to the C9 object identity.

## Scope

Only the original retained programs, capture windows and provider labels in the attached artifacts are covered. Version labels are original receipt metadata, not a fresh patchlevel query. Missing command/API doors and aborted processes stay distinct from successful guest completion. Source files/probes explain the observer protocol; no pinned engine implementation excerpt or executed Rust assertion is claimed. Different attempts and sources are retained separately, never treated as byte-identical duplicates.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.4.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`alias-manifest.json` (`file-10-row-11`):

```json
{
  "version": "8.4.20",
  "exit": 0
}
```

`manifest.json` (`file-17-row-18`):

```json
{
  "release": "8.4.20",
  "exit_code": 0,
  "observations": [
    {
      "code": 0
    },
    {
      "code": 1
    },
    {
      "code": 2
    },
    {
      "code": 3
    },
    {
      "code": 4
    },
    {
      "code": 7
    },
    {
      "code": 0
    },
    {
      "code": 1
    },
    {
      "code": 2
    },
    {
      "code": 3
    },
    {
      "code": 4
    },
    {
      "code": 7
    }
  ]
}
```

`8.4.20.jsonl` (`file-0`):

```json
{
  "original_table": "{\"code\":0,\"seeded\":0,\"result\":\"7600ff\"}\n{\"code\":1,\"seeded\":0,\"result\":\"7600ff\"}\n{\"code\":2,\"seeded\":0,\"result\":\"7600ff\"}\n{\"code\":3,\"seeded\":0,\"result\":\"7600ff\"}\n{\"code\":4,\"seeded\":0,\"result\":\"7600ff\"}\n{\"code\":7,\"seeded\":0,\"result\":\"7600ff\"}\n{\"code\":0,\"seeded\":1,\"result\":\"7600ff\"}\n{\"code\":1,\"seeded\":1,\"result\":\"7600ff\"}\n{\"code\":2,\"seeded\":1,\"result\":\"7600ff\"}\n{\"code\":3,\"seeded\":1,\"result\":\"7600ff\"}\n{\"code\":4,\"seeded\":1,\"result\":\"7600ff\"}\n{\"code\":7,\"seeded\":1,\"result\":\"7600ff\"}\n"
}
```

`alias-8.4.20.jsonl` (`file-5`):

```json
{
  "original_table": "{\"result_same\":1,\"members_same\":1,\"errorinfo_same\":null}\n"
}
```

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.5.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`alias-manifest.json` (`file-10-row-12`):

```json
{
  "version": "8.5.19",
  "exit": 0
}
```

`manifest.json` (`file-17-row-19`):

```json
{
  "release": "8.5.19",
  "exit_code": 0,
  "observations": [
    {
      "code": 0
    },
    {
      "code": 1
    },
    {
      "code": 2
    },
    {
      "code": 3
    },
    {
      "code": 4
    },
    {
      "code": 7
    },
    {
      "code": 0
    },
    {
      "code": 1
    },
    {
      "code": 2
    },
    {
      "code": 3
    },
    {
      "code": 4
    },
    {
      "code": 7
    }
  ]
}
```

`8.5.19.jsonl` (`file-1`):

```json
{
  "original_table": "{\"code\":0,\"seeded\":0,\"result\":\"7600ff\",\"option_code\":\"30\",\"option_level\":\"30\",\"errorcode\":null,\"errorinfo\":null,\"errorline\":null,\"errorstack\":null}\n{\"code\":1,\"seeded\":0,\"result\":\"7600ff\",\"option_code\":\"31\",\"option_level\":\"30\",\"errorcode\":\"4e4f4e45\",\"errorinfo\":\"7600ff\",\"errorline\":\"30\",\"errorstack\":null}\n{\"code\":2,\"seeded\":0,\"result\":\"7600ff\",\"option_code\":\"30\",\"option_level\":\"31\",\"errorcode\":null,\"errorinfo\":null,\"errorline\":null,\"errorstack\":null}\n{\"code\":3,\"seeded\":0,\"result\":\"7600ff\",\"option_code\":\"33\",\"option_level\":\"30\",\"errorcode\":null,\"errorinfo\":null,\"errorline\":null,\"errorstack\":null}\n{\"code\":4,\"seeded\":0,\"result\":\"7600ff\",\"option_code\":\"34\",\"option_level\":\"30\",\"errorcode\":null,\"errorinfo\":null,\"errorline\":null,\"errorstack\":null}\n{\"code\":7,\"seeded\":0,\"result\":\"7600ff\",\"option_code\":\"37\",\"option_level\":\"30\",\"errorcode\":null,\"errorinfo\":null,\"errorline\":null,\"errorstack\":null}\n{\"code\":0,\"seeded\":1,\"result\":\"7600ff\",\"option_code\":\"30\",\"option_level\":\"30\",\"errorcode\":\"435553544f4d2045\",\"errorinfo\":null,\"errorline\":null,\"errorstack\":null}\n{\"code\":1,\"seeded\":1,\"result\":\"7600ff\",\"option_code\":\"31\",\"option_level\":\"30\",\"errorcode\":\"435553544f4d2045\",\"errorinfo\":\"7600ff\",\"errorline\":\"30\",\"errorstack\":null}\n{\"code\":2,\"seeded\":1,\"result\":\"7600ff\",\"option_code\":\"30\",\"option_level\":\"31\",\"errorcode\":\"435553544f4d2045\",\"errorinfo\":null,\"errorline\":null,\"errorstack\":null}\n{\"code\":3,\"seeded\":1,\"result\":\"7600ff\",\"option_code\":\"33\",\"option_level\":\"30\",\"errorcode\":\"435553544f4d2045\",\"errorinfo\":null,\"errorline\":null,\"errorstack\":null}\n{\"code\":4,\"seeded\":1,\"result\":\"7600ff\",\"option_code\":\"34\",\"option_level\":\"30\",\"errorcode\":\"435553544f4d2045\",\"errorinfo\":null,\"errorline\":null,\"errorstack\":null}\n{\"code\":7,\"seeded\":1,\"result\":\"7600ff\",\"option_code\":\"37\",\"option_level\":\"30\",\"errorcode\":\"435553544f4d2045\",\"errorinfo\":null,\"errorline\":null,\"errorstack\":null}\n"
}
```

`alias-8.5.19.jsonl` (`file-6`):

```json
{
  "original_table": "{\"result_same\":1,\"members_same\":1,\"errorinfo_same\":0}\n"
}
```

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.6.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`alias-manifest.json` (`file-10-row-13`):

```json
{
  "version": "8.6.18",
  "exit": 0
}
```

`manifest.json` (`file-17-row-20`):

```json
{
  "release": "8.6.18",
  "exit_code": 0,
  "observations": [
    {
      "code": 0
    },
    {
      "code": 1
    },
    {
      "code": 2
    },
    {
      "code": 3
    },
    {
      "code": 4
    },
    {
      "code": 7
    },
    {
      "code": 0
    },
    {
      "code": 1
    },
    {
      "code": 2
    },
    {
      "code": 3
    },
    {
      "code": 4
    },
    {
      "code": 7
    }
  ]
}
```

`8.6.18.jsonl` (`file-2`):

```json
{
  "original_table": "{\"code\":0,\"seeded\":0,\"result\":\"7600ff\",\"option_code\":\"30\",\"option_level\":\"30\",\"errorcode\":null,\"errorinfo\":null,\"errorline\":null,\"errorstack\":null}\n{\"code\":1,\"seeded\":0,\"result\":\"7600ff\",\"option_code\":\"31\",\"option_level\":\"30\",\"errorcode\":\"4e4f4e45\",\"errorinfo\":\"7600ff\",\"errorline\":\"31\",\"errorstack\":\"\"}\n{\"code\":2,\"seeded\":0,\"result\":\"7600ff\",\"option_code\":\"30\",\"option_level\":\"31\",\"errorcode\":null,\"errorinfo\":null,\"errorline\":null,\"errorstack\":null}\n{\"code\":3,\"seeded\":0,\"result\":\"7600ff\",\"option_code\":\"33\",\"option_level\":\"30\",\"errorcode\":null,\"errorinfo\":null,\"errorline\":null,\"errorstack\":null}\n{\"code\":4,\"seeded\":0,\"result\":\"7600ff\",\"option_code\":\"34\",\"option_level\":\"30\",\"errorcode\":null,\"errorinfo\":null,\"errorline\":null,\"errorstack\":null}\n{\"code\":7,\"seeded\":0,\"result\":\"7600ff\",\"option_code\":\"37\",\"option_level\":\"30\",\"errorcode\":null,\"errorinfo\":null,\"errorline\":null,\"errorstack\":null}\n{\"code\":0,\"seeded\":1,\"result\":\"7600ff\",\"option_code\":\"30\",\"option_level\":\"30\",\"errorcode\":\"435553544f4d2045\",\"errorinfo\":null,\"errorline\":null,\"errorstack\":null}\n{\"code\":1,\"seeded\":1,\"result\":\"7600ff\",\"option_code\":\"31\",\"option_level\":\"30\",\"errorcode\":\"435553544f4d2045\",\"errorinfo\":\"7600ff\",\"errorline\":\"31\",\"errorstack\":\"\"}\n{\"code\":2,\"seeded\":1,\"result\":\"7600ff\",\"option_code\":\"30\",\"option_level\":\"31\",\"errorcode\":\"435553544f4d2045\",\"errorinfo\":null,\"errorline\":null,\"errorstack\":null}\n{\"code\":3,\"seeded\":1,\"result\":\"7600ff\",\"option_code\":\"33\",\"option_level\":\"30\",\"errorcode\":\"435553544f4d2045\",\"errorinfo\":null,\"errorline\":null,\"errorstack\":null}\n{\"code\":4,\"seeded\":1,\"result\":\"7600ff\",\"option_code\":\"34\",\"option_level\":\"30\",\"errorcode\":\"435553544f4d2045\",\"errorinfo\":null,\"errorline\":null,\"errorstack\":null}\n{\"code\":7,\"seeded\":1,\"result\":\"7600ff\",\"option_code\":\"37\",\"option_level\":\"30\",\"errorcode\":\"435553544f4d2045\",\"errorinfo\":null,\"errorline\":null,\"errorstack\":null}\n"
}
```

`alias-8.6.18.jsonl` (`file-7`):

```json
{
  "original_table": "{\"result_same\":1,\"members_same\":1,\"errorinfo_same\":0}\n"
}
```

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl9.0.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`alias-manifest.json` (`file-10-row-14`):

```json
{
  "version": "9.0.4",
  "exit": 0
}
```

`manifest.json` (`file-17-row-21`):

```json
{
  "release": "9.0.4",
  "exit_code": 0,
  "observations": [
    {
      "code": 0
    },
    {
      "code": 1
    },
    {
      "code": 2
    },
    {
      "code": 3
    },
    {
      "code": 4
    },
    {
      "code": 7
    },
    {
      "code": 0
    },
    {
      "code": 1
    },
    {
      "code": 2
    },
    {
      "code": 3
    },
    {
      "code": 4
    },
    {
      "code": 7
    }
  ]
}
```

`9.0.4.jsonl` (`file-3`):

```json
{
  "original_table": "{\"code\":0,\"seeded\":0,\"result\":\"7600ff\",\"option_code\":\"30\",\"option_level\":\"30\",\"errorcode\":null,\"errorinfo\":null,\"errorline\":null,\"errorstack\":null}\n{\"code\":1,\"seeded\":0,\"result\":\"7600ff\",\"option_code\":\"31\",\"option_level\":\"30\",\"errorcode\":\"4e4f4e45\",\"errorinfo\":\"7600ff\",\"errorline\":\"31\",\"errorstack\":\"\"}\n{\"code\":2,\"seeded\":0,\"result\":\"7600ff\",\"option_code\":\"30\",\"option_level\":\"31\",\"errorcode\":null,\"errorinfo\":null,\"errorline\":null,\"errorstack\":null}\n{\"code\":3,\"seeded\":0,\"result\":\"7600ff\",\"option_code\":\"33\",\"option_level\":\"30\",\"errorcode\":null,\"errorinfo\":null,\"errorline\":null,\"errorstack\":null}\n{\"code\":4,\"seeded\":0,\"result\":\"7600ff\",\"option_code\":\"34\",\"option_level\":\"30\",\"errorcode\":null,\"errorinfo\":null,\"errorline\":null,\"errorstack\":null}\n{\"code\":7,\"seeded\":0,\"result\":\"7600ff\",\"option_code\":\"37\",\"option_level\":\"30\",\"errorcode\":null,\"errorinfo\":null,\"errorline\":null,\"errorstack\":null}\n{\"code\":0,\"seeded\":1,\"result\":\"7600ff\",\"option_code\":\"30\",\"option_level\":\"30\",\"errorcode\":\"435553544f4d2045\",\"errorinfo\":null,\"errorline\":null,\"errorstack\":null}\n{\"code\":1,\"seeded\":1,\"result\":\"7600ff\",\"option_code\":\"31\",\"option_level\":\"30\",\"errorcode\":\"435553544f4d2045\",\"errorinfo\":\"7600ff\",\"errorline\":\"31\",\"errorstack\":\"\"}\n{\"code\":2,\"seeded\":1,\"result\":\"7600ff\",\"option_code\":\"30\",\"option_level\":\"31\",\"errorcode\":\"435553544f4d2045\",\"errorinfo\":null,\"errorline\":null,\"errorstack\":null}\n{\"code\":3,\"seeded\":1,\"result\":\"7600ff\",\"option_code\":\"33\",\"option_level\":\"30\",\"errorcode\":\"435553544f4d2045\",\"errorinfo\":null,\"errorline\":null,\"errorstack\":null}\n{\"code\":4,\"seeded\":1,\"result\":\"7600ff\",\"option_code\":\"34\",\"option_level\":\"30\",\"errorcode\":\"435553544f4d2045\",\"errorinfo\":null,\"errorline\":null,\"errorstack\":null}\n{\"code\":7,\"seeded\":1,\"result\":\"7600ff\",\"option_code\":\"37\",\"option_level\":\"30\",\"errorcode\":\"435553544f4d2045\",\"errorinfo\":null,\"errorline\":null,\"errorstack\":null}\n"
}
```

`alias-9.0.4.jsonl` (`file-8`):

```json
{
  "original_table": "{\"result_same\":1,\"members_same\":1,\"errorinfo_same\":1}\n"
}
```

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl9.1.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`alias-manifest.json` (`file-10-row-15`):

```json
{
  "version": "9.1.0",
  "exit": 0
}
```

`manifest.json` (`file-17-row-22`):

```json
{
  "release": "9.1.0",
  "exit_code": 0,
  "observations": [
    {
      "code": 0
    },
    {
      "code": 1
    },
    {
      "code": 2
    },
    {
      "code": 3
    },
    {
      "code": 4
    },
    {
      "code": 7
    },
    {
      "code": 0
    },
    {
      "code": 1
    },
    {
      "code": 2
    },
    {
      "code": 3
    },
    {
      "code": 4
    },
    {
      "code": 7
    }
  ]
}
```

`9.1.0.jsonl` (`file-4`):

```json
{
  "original_table": "{\"code\":0,\"seeded\":0,\"result\":\"7600ff\",\"option_code\":\"30\",\"option_level\":\"30\",\"errorcode\":null,\"errorinfo\":null,\"errorline\":null,\"errorstack\":null}\n{\"code\":1,\"seeded\":0,\"result\":\"7600ff\",\"option_code\":\"31\",\"option_level\":\"30\",\"errorcode\":\"4e4f4e45\",\"errorinfo\":\"7600ff\",\"errorline\":\"31\",\"errorstack\":\"\"}\n{\"code\":2,\"seeded\":0,\"result\":\"7600ff\",\"option_code\":\"30\",\"option_level\":\"31\",\"errorcode\":null,\"errorinfo\":null,\"errorline\":null,\"errorstack\":null}\n{\"code\":3,\"seeded\":0,\"result\":\"7600ff\",\"option_code\":\"33\",\"option_level\":\"30\",\"errorcode\":null,\"errorinfo\":null,\"errorline\":null,\"errorstack\":null}\n{\"code\":4,\"seeded\":0,\"result\":\"7600ff\",\"option_code\":\"34\",\"option_level\":\"30\",\"errorcode\":null,\"errorinfo\":null,\"errorline\":null,\"errorstack\":null}\n{\"code\":7,\"seeded\":0,\"result\":\"7600ff\",\"option_code\":\"37\",\"option_level\":\"30\",\"errorcode\":null,\"errorinfo\":null,\"errorline\":null,\"errorstack\":null}\n{\"code\":0,\"seeded\":1,\"result\":\"7600ff\",\"option_code\":\"30\",\"option_level\":\"30\",\"errorcode\":\"435553544f4d2045\",\"errorinfo\":null,\"errorline\":null,\"errorstack\":null}\n{\"code\":1,\"seeded\":1,\"result\":\"7600ff\",\"option_code\":\"31\",\"option_level\":\"30\",\"errorcode\":\"435553544f4d2045\",\"errorinfo\":\"7600ff\",\"errorline\":\"31\",\"errorstack\":\"\"}\n{\"code\":2,\"seeded\":1,\"result\":\"7600ff\",\"option_code\":\"30\",\"option_level\":\"31\",\"errorcode\":\"435553544f4d2045\",\"errorinfo\":null,\"errorline\":null,\"errorstack\":null}\n{\"code\":3,\"seeded\":1,\"result\":\"7600ff\",\"option_code\":\"33\",\"option_level\":\"30\",\"errorcode\":\"435553544f4d2045\",\"errorinfo\":null,\"errorline\":null,\"errorstack\":null}\n{\"code\":4,\"seeded\":1,\"result\":\"7600ff\",\"option_code\":\"34\",\"option_level\":\"30\",\"errorcode\":\"435553544f4d2045\",\"errorinfo\":null,\"errorline\":null,\"errorstack\":null}\n{\"code\":7,\"seeded\":1,\"result\":\"7600ff\",\"option_code\":\"37\",\"option_level\":\"30\",\"errorcode\":\"435553544f4d2045\",\"errorinfo\":null,\"errorline\":null,\"errorstack\":null}\n"
}
```

`alias-9.1.0.jsonl` (`file-9`):

```json
{
  "original_table": "{\"result_same\":1,\"members_same\":1,\"errorinfo_same\":1}\n"
}
```

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: jim.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: bigip.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Historical references

- `file-0` (observation): [rust/tcl-cshim/tests/data/native_callback_completions/8.4.20.jsonl](../../../../rust/tcl-cshim/tests/data/native_callback_completions/8.4.20.jsonl). Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-1` (observation): [rust/tcl-cshim/tests/data/native_callback_completions/8.5.19.jsonl](../../../../rust/tcl-cshim/tests/data/native_callback_completions/8.5.19.jsonl). Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-2` (observation): [rust/tcl-cshim/tests/data/native_callback_completions/8.6.18.jsonl](../../../../rust/tcl-cshim/tests/data/native_callback_completions/8.6.18.jsonl). Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-3` (observation): [rust/tcl-cshim/tests/data/native_callback_completions/9.0.4.jsonl](../../../../rust/tcl-cshim/tests/data/native_callback_completions/9.0.4.jsonl). Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-4` (observation): [rust/tcl-cshim/tests/data/native_callback_completions/9.1.0.jsonl](../../../../rust/tcl-cshim/tests/data/native_callback_completions/9.1.0.jsonl). Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-5` (observation): [rust/tcl-cshim/tests/data/native_callback_completions/alias-8.4.20.jsonl](../../../../rust/tcl-cshim/tests/data/native_callback_completions/alias-8.4.20.jsonl). Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-6` (observation): [rust/tcl-cshim/tests/data/native_callback_completions/alias-8.5.19.jsonl](../../../../rust/tcl-cshim/tests/data/native_callback_completions/alias-8.5.19.jsonl). Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-7` (observation): [rust/tcl-cshim/tests/data/native_callback_completions/alias-8.6.18.jsonl](../../../../rust/tcl-cshim/tests/data/native_callback_completions/alias-8.6.18.jsonl). Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-8` (observation): [rust/tcl-cshim/tests/data/native_callback_completions/alias-9.0.4.jsonl](../../../../rust/tcl-cshim/tests/data/native_callback_completions/alias-9.0.4.jsonl). Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-9` (observation): [rust/tcl-cshim/tests/data/native_callback_completions/alias-9.1.0.jsonl](../../../../rust/tcl-cshim/tests/data/native_callback_completions/alias-9.1.0.jsonl). Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-10` (observation): [rust/tcl-cshim/tests/data/native_callback_completions/alias-manifest.json](../../../../rust/tcl-cshim/tests/data/native_callback_completions/alias-manifest.json). Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-10-row-11` (provider): [rust/tcl-cshim/tests/data/native_callback_completions/alias-manifest.json](../../../../rust/tcl-cshim/tests/data/native_callback_completions/alias-manifest.json). JSON pointer `/0`. Original provider/capture association at its exact selected row.
- `file-10-row-12` (provider): [rust/tcl-cshim/tests/data/native_callback_completions/alias-manifest.json](../../../../rust/tcl-cshim/tests/data/native_callback_completions/alias-manifest.json). JSON pointer `/1`. Original provider/capture association at its exact selected row.
- `file-10-row-13` (provider): [rust/tcl-cshim/tests/data/native_callback_completions/alias-manifest.json](../../../../rust/tcl-cshim/tests/data/native_callback_completions/alias-manifest.json). JSON pointer `/2`. Original provider/capture association at its exact selected row.
- `file-10-row-14` (provider): [rust/tcl-cshim/tests/data/native_callback_completions/alias-manifest.json](../../../../rust/tcl-cshim/tests/data/native_callback_completions/alias-manifest.json). JSON pointer `/3`. Original provider/capture association at its exact selected row.
- `file-10-row-15` (provider): [rust/tcl-cshim/tests/data/native_callback_completions/alias-manifest.json](../../../../rust/tcl-cshim/tests/data/native_callback_completions/alias-manifest.json). JSON pointer `/4`. Original provider/capture association at its exact selected row.
- `file-16` (input): [rust/tcl-cshim/tests/data/native_callback_completions/alias.c](../../../../rust/tcl-cshim/tests/data/native_callback_completions/alias.c). Exact retained input/program bytes; purpose is limited to this question.
- `file-17` (observation): [rust/tcl-cshim/tests/data/native_callback_completions/manifest.json](../../../../rust/tcl-cshim/tests/data/native_callback_completions/manifest.json). Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-17-row-18` (provider): [rust/tcl-cshim/tests/data/native_callback_completions/manifest.json](../../../../rust/tcl-cshim/tests/data/native_callback_completions/manifest.json). JSON pointer `/0`. Original provider/capture association at its exact selected row.
- `file-17-row-19` (provider): [rust/tcl-cshim/tests/data/native_callback_completions/manifest.json](../../../../rust/tcl-cshim/tests/data/native_callback_completions/manifest.json). JSON pointer `/1`. Original provider/capture association at its exact selected row.
- `file-17-row-20` (provider): [rust/tcl-cshim/tests/data/native_callback_completions/manifest.json](../../../../rust/tcl-cshim/tests/data/native_callback_completions/manifest.json). JSON pointer `/2`. Original provider/capture association at its exact selected row.
- `file-17-row-21` (provider): [rust/tcl-cshim/tests/data/native_callback_completions/manifest.json](../../../../rust/tcl-cshim/tests/data/native_callback_completions/manifest.json). JSON pointer `/3`. Original provider/capture association at its exact selected row.
- `file-17-row-22` (provider): [rust/tcl-cshim/tests/data/native_callback_completions/manifest.json](../../../../rust/tcl-cshim/tests/data/native_callback_completions/manifest.json). JSON pointer `/4`. Original provider/capture association at its exact selected row.
- `file-23` (input): [rust/tcl-cshim/tests/data/native_callback_completions/probe.c](../../../../rust/tcl-cshim/tests/data/native_callback_completions/probe.c). Exact retained input/program bytes; purpose is limited to this question.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The attached artifacts retain captured outputs, available original inputs and recorded compile/run arguments. A missing input or wrapper limits exact replay; no new native execution is claimed. A replay requires the corresponding source/library/build configuration and original evaluator flags/argv; vanished outside capture paths are provenance, not executable replay dependencies. Where only source hex is retained, preserve those bytes. The per-purpose README/probe supplies the protocol; no new accepted reconfirmation or Rust pass is asserted.
