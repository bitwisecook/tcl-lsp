# BIG-IP stock Tcl: string-prefix-near-miss

## Question

What happens when a command name containing spaces is interpolated into an eval script string rather than preserved as a list command-prefix element?

This case informs name resolution and diagnostics in the LSP, compiler and VM; it does not establish behavior in other environments.

## Assumptions

Recorded case scope: Original authored names and concatenated source bytes; setup/final command split in a fresh stock process. Process destruction supplies interpreter isolation; no in-process cleanup. This is a wrapper/frame/catch-boundary adaptation, not an exact portable-input answer.

Other assumptions were not separately recorded. Connection identifiers were redacted in the retained invocation.

## Script

Fixture: [bigip-stock-8.4-1](fixtures/bigip-stock-8.4-1.md)

- run: `nh20261012pat3`
- case: `string-prefix-near-miss`

### nh_setup

```tcl-visible
namespace eval ::nh {}
proc {::nh::two words} {fixed value} {list $fixed $value}
set target {::nh::two words}
```

### nh_operation

```tcl-visible
eval "$target {a b} {c d}"

```

## Results

### BIG-IP stock-8.4

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: SSH to /usr/bin/tclsh8.4, script on standard input.
- Exit status: 0
- Standard output: decoded below.

- Standard error: empty.

The exit status and streams are from the SSH invocation; standard error can include SSH transport messages as well as remote errors. The entries below decode complete NHO result payloads from standard output.

- setup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
::nh::two words⟦EOF⟧
```

- lookup, completion 1, result (utf-8; unit none, invocation once):

```text-visible
invalid command name "::nh::two"⟦EOF⟧
```

- lookup, completion 1, metadata (utf-8; unit none, invocation once):

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- lookup, completion 1, errorcode (utf-8; unit none, invocation once):

```text-visible
NONE⟦EOF⟧
```

- lookup, completion 1, errorinfo (utf-8; unit none, invocation once):

```text-visible
invalid command name "::nh::two"
    while executing
"::nh::two words {a b} {c d}"
    ("eval" body line 1)
    invoked from within
"eval "$target {a b} {c d}""
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```
