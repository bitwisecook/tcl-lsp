# BIG-IP stock Tcl: keyed-variable-link

## Question

Can upvar link to a namespace array whose constructed name contains a space, and does writing an element through the link update the intended namespace variable?

This case informs name resolution and diagnostics in the LSP, compiler and VM; it does not establish behavior in other environments.

## Assumptions

Recorded case scope: Original authored names and concatenated source bytes; setup/final command split in a fresh stock process. Process destruction supplies interpreter isolation; no in-process cleanup. This is a wrapper/frame/catch-boundary adaptation, not an exact portable-input answer.

Other assumptions were not separately recorded. Connection identifiers were redacted in the retained invocation.

## Script

Fixture: [bigip-stock-8.5-1](fixtures/bigip-stock-8.5-1.md)

- run: `nh20261012pat3`
- case: `keyed-variable-link`

### nh_setup

```tcl-visible
namespace eval ::nh {
    set {state.a b(value)} original
    proc change {name} {
        upvar ::nh::$name state
        set state(value) changed
    }
}
::nh::change {state.a b}
```

### nh_operation

```tcl-visible
set {::nh::state.a b(value)}

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
changed⟦EOF⟧
```

- setup, completion 0, returnoptions (utf-8; unit none, invocation once):

```text-visible
-code 0 -level 0⟦EOF⟧
```

- lookup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
changed⟦EOF⟧
```

- lookup, completion 0, returnoptions (utf-8; unit none, invocation once):

```text-visible
-code 0 -level 0⟦EOF⟧
```
