# naming.error.original-log-options-and-special-frame-extents

Kind: `native-observation`

## Problem statement

Error stack and log extents depend on catch/unwind/return and special frame entry. A final message or inferred source offset cannot attest original return-options objects or frame diagnostics.

## Question

What original error log, return-options and special-frame source extents do the three independent probes retain?

## Conclusion

Script error logs, public return-options calls and special-frame probes retain distinct original frontiers and release availability. Tcl 8.4 options absence is explicit. No output supplies an unobserved caller frame, callback or object header from another probe.

## Scope

Only the original retained programs, capture windows and provider labels in the attached artifacts are covered. Version labels are original receipt metadata, not a fresh patchlevel query. Missing command/API doors and aborted processes stay distinct from successful guest completion. Source files/probes explain the observer protocol; no pinned engine implementation excerpt or executed Rust assertion is claimed. Different attempts and sources are retained separately, never treated as byte-identical duplicates.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_EvalEx, Tcl_EvalObjv. Exact lengths, flags and input constructors remain in the source. Version: tcl8.4 8.4.20.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`return_options_provenance.json` (`file-8-row-9`):

```json
{
  "version": "8.4.20",
  "compile_exit": 0,
  "exit": 0,
  "observations": [
    "error|1|unavailable|unavailable",
    "explicit-error|1|unavailable|unavailable",
    "return|1|unavailable|unavailable",
    "custom|1|unavailable|unavailable"
  ]
}
```

`return_options.txt` (`file-7`):

```json
{
  "provider_rows": [
    "8.4.20|error|1|unavailable|unavailable",
    "8.4.20|explicit-error|1|unavailable|unavailable",
    "8.4.20|return|1|unavailable|unavailable",
    "8.4.20|custom|1|unavailable|unavailable"
  ]
}
```

### tcl8.5

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_EvalEx, Tcl_EvalObjv. Exact lengths, flags and input constructors remain in the source. Version: tcl8.5 8.5.19.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`return_options_provenance.json` (`file-8-row-10`):

```json
{
  "version": "8.5.19",
  "compile_exit": 0,
  "exit": 0,
  "observations": [
    "error|1||4e4f4e45",
    "explicit-error|1|2d6572726f72696e666f20494e464f202d6572726f72636f646520435553544f4d|435553544f4d",
    "return|1||4e4f4e45",
    "custom|1|2d637573746f6d206b657074202d6572726f72636f646520435553544f4d|435553544f4d"
  ]
}
```

`return_options.txt` (`file-7`):

```json
{
  "provider_rows": [
    "8.5.19|error|1||4e4f4e45",
    "8.5.19|explicit-error|1|2d6572726f72696e666f20494e464f202d6572726f72636f646520435553544f4d|435553544f4d",
    "8.5.19|return|1||4e4f4e45",
    "8.5.19|custom|1|2d637573746f6d206b657074202d6572726f72636f646520435553544f4d|435553544f4d"
  ]
}
```

### tcl8.6

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_EvalEx, Tcl_EvalObjv. Exact lengths, flags and input constructors remain in the source. Version: tcl8.6 8.6; 8.6.18.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`provenance.json` (`file-2-row-3`):

```json
{
  "version": "8.6",
  "exit": 0,
  "stdout": "caught|{INNER {returnImm BODY {}} CALL p} {-code -level -errorstack -errorcode -errorinfo -errorline}\nunwound|{INNER {returnImm BODY {}} CALL q} {-code -level -errorstack -errorcode -errorinfo -errorline}\nreturn|{INNER {invokeStk1 r}} {-code -level -errorstack -errorcode -errorinfo -errorline}\nshifted|{INNER {returnImm BODY {}} UP 1 CALL u} {-code -level -errorstack -errorcode -errorinfo -errorline}\nexplicit|{INNER {returnImm BODY {-custom kept -errorcode CUSTOM}} CALL e} {-custom -errorcode -code -level -errorstack -errorinfo -errorline}\n",
  "stderr": ""
}
```

`return_options_provenance.json` (`file-8-row-11`):

```json
{
  "version": "8.6.18",
  "compile_exit": 0,
  "exit": 0,
  "observations": [
    "error|1||4e4f4e45",
    "explicit-error|1|2d6572726f72696e666f20494e464f202d6572726f72636f646520435553544f4d|435553544f4d",
    "return|1||4e4f4e45",
    "custom|1|2d637573746f6d206b657074202d6572726f72636f646520435553544f4d|435553544f4d"
  ]
}
```

`special_frames_provenance.json` (`file-16-row-17`):

```json
{
  "version": "8.6.18",
  "compile_exit": 0,
  "exit": 0,
  "observations": [
    "root|0|{INNER {returnImm BODY {}}} {-code -level -errorstack -errorcode -errorinfo -errorline}",
    "special-shift|0|{INNER {returnImm BODY {}}} {-code -level -errorstack -errorcode -errorinfo -errorline}",
    "special-same|0|{INNER {returnImm BODY {}}} {-code -level -errorstack -errorcode -errorinfo -errorline}",
    "nested-shift|0|{INNER {returnImm BODY {}} UP 2 CALL p} {-code -level -errorstack -errorcode -errorinfo -errorline}"
  ]
}
```

`observations.txt` (`file-0`):

```json
{
  "provider_rows": [
    "8.6|caught|{INNER {returnImm BODY {}} CALL p} {-code -level -errorstack -errorcode -errorinfo -errorline}",
    "8.6|unwound|{INNER {returnImm BODY {}} CALL q} {-code -level -errorstack -errorcode -errorinfo -errorline}",
    "8.6|return|{INNER {invokeStk1 r}} {-code -level -errorstack -errorcode -errorinfo -errorline}",
    "8.6|shifted|{INNER {returnImm BODY {}} UP 1 CALL u} {-code -level -errorstack -errorcode -errorinfo -errorline}",
    "8.6|explicit|{INNER {returnImm BODY {-custom kept -errorcode CUSTOM}} CALL e} {-custom -errorcode -code -level -errorstack -errorinfo -errorline}"
  ]
}
```

`return_options.txt` (`file-7`):

```json
{
  "provider_rows": [
    "8.6.18|error|1||4e4f4e45",
    "8.6.18|explicit-error|1|2d6572726f72696e666f20494e464f202d6572726f72636f646520435553544f4d|435553544f4d",
    "8.6.18|return|1||4e4f4e45",
    "8.6.18|custom|1|2d637573746f6d206b657074202d6572726f72636f646520435553544f4d|435553544f4d"
  ]
}
```

`special_frames.txt` (`file-15`):

```json
{
  "provider_rows": [
    "8.6.18|root|0|{INNER {returnImm BODY {}}} {-code -level -errorstack -errorcode -errorinfo -errorline}",
    "8.6.18|special-shift|0|{INNER {returnImm BODY {}}} {-code -level -errorstack -errorcode -errorinfo -errorline}",
    "8.6.18|special-same|0|{INNER {returnImm BODY {}}} {-code -level -errorstack -errorcode -errorinfo -errorline}",
    "8.6.18|nested-shift|0|{INNER {returnImm BODY {}} UP 2 CALL p} {-code -level -errorstack -errorcode -errorinfo -errorline}"
  ]
}
```

### tcl9.0

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_EvalEx, Tcl_EvalObjv. Exact lengths, flags and input constructors remain in the source. Version: tcl9.0 9.0; 9.0.4.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`provenance.json` (`file-2-row-4`):

```json
{
  "version": "9.0",
  "exit": 0,
  "stdout": "caught|{INNER {returnImm BODY {}} CALL p} {-code -level -errorstack -errorcode -errorinfo -errorline}\nunwound|{INNER {returnImm BODY {}} CALL q} {-code -level -errorstack -errorcode -errorinfo -errorline}\nreturn|{INNER {invokeStk1 r}} {-code -level -errorstack -errorcode -errorinfo -errorline}\nshifted|{INNER {returnImm BODY {}} UP 1 CALL u} {-code -level -errorstack -errorcode -errorinfo -errorline}\nexplicit|{INNER {returnImm BODY {-custom kept -errorcode CUSTOM}} CALL e} {-custom -errorcode -code -level -errorstack -errorinfo -errorline}\n",
  "stderr": ""
}
```

`return_options_provenance.json` (`file-8-row-12`):

```json
{
  "version": "9.0.4",
  "compile_exit": 0,
  "exit": 0,
  "observations": [
    "error|1||4e4f4e45",
    "explicit-error|1|2d6572726f72696e666f20494e464f202d6572726f72636f646520435553544f4d|435553544f4d",
    "return|1||4e4f4e45",
    "custom|1|2d637573746f6d206b657074202d6572726f72636f646520435553544f4d|435553544f4d"
  ]
}
```

`special_frames_provenance.json` (`file-16-row-18`):

```json
{
  "version": "9.0.4",
  "compile_exit": 0,
  "exit": 0,
  "observations": [
    "root|0|{INNER {returnImm BODY {}}} {-code -level -errorstack -errorcode -errorinfo -errorline}",
    "special-shift|0|{INNER {returnImm BODY {}}} {-code -level -errorstack -errorcode -errorinfo -errorline}",
    "special-same|0|{INNER {returnImm BODY {}}} {-code -level -errorstack -errorcode -errorinfo -errorline}",
    "nested-shift|0|{INNER {returnImm BODY {}} UP 2 CALL p} {-code -level -errorstack -errorcode -errorinfo -errorline}"
  ]
}
```

`observations.txt` (`file-0`):

```json
{
  "provider_rows": [
    "9.0|caught|{INNER {returnImm BODY {}} CALL p} {-code -level -errorstack -errorcode -errorinfo -errorline}",
    "9.0|unwound|{INNER {returnImm BODY {}} CALL q} {-code -level -errorstack -errorcode -errorinfo -errorline}",
    "9.0|return|{INNER {invokeStk1 r}} {-code -level -errorstack -errorcode -errorinfo -errorline}",
    "9.0|shifted|{INNER {returnImm BODY {}} UP 1 CALL u} {-code -level -errorstack -errorcode -errorinfo -errorline}",
    "9.0|explicit|{INNER {returnImm BODY {-custom kept -errorcode CUSTOM}} CALL e} {-custom -errorcode -code -level -errorstack -errorinfo -errorline}"
  ]
}
```

`return_options.txt` (`file-7`):

```json
{
  "provider_rows": [
    "9.0.4|error|1||4e4f4e45",
    "9.0.4|explicit-error|1|2d6572726f72696e666f20494e464f202d6572726f72636f646520435553544f4d|435553544f4d",
    "9.0.4|return|1||4e4f4e45",
    "9.0.4|custom|1|2d637573746f6d206b657074202d6572726f72636f646520435553544f4d|435553544f4d"
  ]
}
```

`special_frames.txt` (`file-15`):

```json
{
  "provider_rows": [
    "9.0.4|root|0|{INNER {returnImm BODY {}}} {-code -level -errorstack -errorcode -errorinfo -errorline}",
    "9.0.4|special-shift|0|{INNER {returnImm BODY {}}} {-code -level -errorstack -errorcode -errorinfo -errorline}",
    "9.0.4|special-same|0|{INNER {returnImm BODY {}}} {-code -level -errorstack -errorcode -errorinfo -errorline}",
    "9.0.4|nested-shift|0|{INNER {returnImm BODY {}} UP 2 CALL p} {-code -level -errorstack -errorcode -errorinfo -errorline}"
  ]
}
```

### tcl9.1

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_EvalEx, Tcl_EvalObjv. Exact lengths, flags and input constructors remain in the source. Version: tcl9.1 9.1; 9.1.0.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`provenance.json` (`file-2-row-5`):

```json
{
  "version": "9.1",
  "exit": 0,
  "stdout": "caught|{INNER {returnImm BODY {}} CALL p} {-code -level -errorstack -errorcode -errorinfo -errorline}\nunwound|{INNER {returnImm BODY {}} CALL q} {-code -level -errorstack -errorcode -errorinfo -errorline}\nreturn|{INNER {invokeStk r}} {-code -level -errorstack -errorcode -errorinfo -errorline}\nshifted|{INNER {returnImm BODY {}} UP 1 CALL u} {-code -level -errorstack -errorcode -errorinfo -errorline}\nexplicit|{INNER {returnImm BODY {-custom kept -errorcode CUSTOM}} CALL e} {-custom -errorcode -code -level -errorstack -errorinfo -errorline}\n",
  "stderr": ""
}
```

`return_options_provenance.json` (`file-8-row-13`):

```json
{
  "version": "9.1.0",
  "compile_exit": 0,
  "exit": 0,
  "observations": [
    "error|1||4e4f4e45",
    "explicit-error|1|2d6572726f72696e666f20494e464f202d6572726f72636f646520435553544f4d|435553544f4d",
    "return|1||4e4f4e45",
    "custom|1|2d637573746f6d206b657074202d6572726f72636f646520435553544f4d|435553544f4d"
  ]
}
```

`special_frames_provenance.json` (`file-16-row-19`):

```json
{
  "version": "9.1.0",
  "compile_exit": 0,
  "exit": 0,
  "observations": [
    "root|0|{INNER {returnImm BODY {}}} {-code -level -errorstack -errorcode -errorinfo -errorline}",
    "special-shift|0|{INNER {returnImm BODY {}}} {-code -level -errorstack -errorcode -errorinfo -errorline}",
    "special-same|0|{INNER {returnImm BODY {}}} {-code -level -errorstack -errorcode -errorinfo -errorline}",
    "nested-shift|0|{INNER {returnImm BODY {}} UP 2 CALL p} {-code -level -errorstack -errorcode -errorinfo -errorline}"
  ]
}
```

`observations.txt` (`file-0`):

```json
{
  "provider_rows": [
    "9.1|caught|{INNER {returnImm BODY {}} CALL p} {-code -level -errorstack -errorcode -errorinfo -errorline}",
    "9.1|unwound|{INNER {returnImm BODY {}} CALL q} {-code -level -errorstack -errorcode -errorinfo -errorline}",
    "9.1|return|{INNER {invokeStk r}} {-code -level -errorstack -errorcode -errorinfo -errorline}",
    "9.1|shifted|{INNER {returnImm BODY {}} UP 1 CALL u} {-code -level -errorstack -errorcode -errorinfo -errorline}",
    "9.1|explicit|{INNER {returnImm BODY {-custom kept -errorcode CUSTOM}} CALL e} {-custom -errorcode -code -level -errorstack -errorinfo -errorline}"
  ]
}
```

`return_options.txt` (`file-7`):

```json
{
  "provider_rows": [
    "9.1.0|error|1||4e4f4e45",
    "9.1.0|explicit-error|1|2d6572726f72696e666f20494e464f202d6572726f72636f646520435553544f4d|435553544f4d",
    "9.1.0|return|1||4e4f4e45",
    "9.1.0|custom|1|2d637573746f6d206b657074202d6572726f72636f646520435553544f4d|435553544f4d"
  ]
}
```

`special_frames.txt` (`file-15`):

```json
{
  "provider_rows": [
    "9.1.0|root|0|{INNER {returnImm BODY {}}} {-code -level -errorstack -errorcode -errorinfo -errorline}",
    "9.1.0|special-shift|0|{INNER {returnImm BODY {}}} {-code -level -errorstack -errorcode -errorinfo -errorline}",
    "9.1.0|special-same|0|{INNER {returnImm BODY {}}} {-code -level -errorstack -errorcode -errorinfo -errorline}",
    "9.1.0|nested-shift|0|{INNER {returnImm BODY {}} UP 2 CALL p} {-code -level -errorstack -errorcode -errorinfo -errorline}"
  ]
}
```

### jim, bigip

Status: `not-tested`.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-syntax/tests/data/native_error_log](../../../../rust/tcl-syntax/tests/data/native_error_log)
