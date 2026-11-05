# BIG-IP stock Tcl: computed-qualified

## Question

Can a qualified command name assembled from a namespace variable be invoked as a computed command head, and which command does that spelling reach?

This case informs name resolution and diagnostics in the LSP, compiler and VM; it does not establish behavior in other environments.

## Assumptions

Recorded case scope: Original authored names and concatenated source bytes; setup/final command split in a fresh stock process. Process destruction supplies interpreter isolation; no in-process cleanup. This is a wrapper/frame/catch-boundary adaptation, not an exact portable-input answer.

Other assumptions were not separately recorded. Connection identifiers were redacted in the retained invocation.

## Script

Fixture: [bigip-stock-8.5-1](fixtures/bigip-stock-8.5-1.md)

- run: `nh20261012stock3`
- case: `computed-qualified`

### nh_setup

```tcl-visible
namespace eval ::nh {proc emit {} {return first}}
set ns ::nh
set head ${ns}::emit
```

### nh_operation

```tcl-visible
$head

```

## Results

### BIG-IP stock-8.5

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: SSH to /usr/bin/tclsh8.5, script on standard input.
- Exit status: 0
- Standard output: decoded below.

- Standard error: empty.

The exit status and streams are from the SSH invocation; standard error can include SSH transport messages as well as remote errors. The entries below decode complete NHO result payloads from standard output.

- setup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
::nh::emit⟦EOF⟧
```

- setup, completion 0, returnoptions (utf-8; unit none, invocation once):

```text-visible
-code 0 -level 0⟦EOF⟧
```

- lookup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
first⟦EOF⟧
```

- lookup, completion 0, returnoptions (utf-8; unit none, invocation once):

```text-visible
-code 0 -level 0⟦EOF⟧
```
