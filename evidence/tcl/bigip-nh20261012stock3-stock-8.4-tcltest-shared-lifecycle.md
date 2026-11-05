# BIG-IP stock Tcl: tcltest-shared-lifecycle

## Question

Do tcltest setup, body and cleanup share the expected namespace variable state, and is the setup variable absent after cleanup?

This case informs name resolution and diagnostics in the LSP, compiler and VM; it does not establish behavior in other environments.

## Assumptions

Recorded case scope: Original authored names and concatenated source bytes; setup/final command split in a fresh stock process. Process destruction supplies interpreter isolation; no in-process cleanup. This is a wrapper/frame/catch-boundary adaptation, not an exact portable-input answer.

Other assumptions were not separately recorded. Connection identifiers were redacted in the retained invocation.

## Script

Fixture: [bigip-stock-8.4-1](fixtures/bigip-stock-8.4-1.md)

- run: `nh20261012stock3`
- case: `tcltest-shared-lifecycle`

### nh_setup

```tcl-visible
package require tcltest
namespace eval ::nh {
    namespace import ::tcltest::*
    set observed {}
    test name-corpus-1 {} -setup {
        set payload {a b}
    } -body {
        lappend observed [set payload]
    } -cleanup {
        lappend observed [set payload]
        unset payload
    } -result {{a b}}
}
```

### nh_operation

```tcl-visible
list $::nh::observed [info exists ::nh::payload]

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
⟦EOF⟧
```

- lookup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
{{a b} {a b}} 0⟦EOF⟧
```
