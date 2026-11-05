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

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source. Version: tcl8.4 8.4.20.

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

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source. Version: tcl8.5 8.5.19.

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

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source. Version: tcl8.6 8.6.18.

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

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source. Version: tcl9.0 9.0.4.

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

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source. Version: tcl9.1 9.1.0.

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

### jim, bigip

Status: `not-tested`.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-cshim/tests/data/native_callback_completions](../../../../rust/tcl-cshim/tests/data/native_callback_completions)
