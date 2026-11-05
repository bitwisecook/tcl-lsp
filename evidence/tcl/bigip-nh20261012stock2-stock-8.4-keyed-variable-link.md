# BIG-IP stock Tcl: keyed-variable-link

## Question

Can upvar link to a namespace array whose constructed name contains a space, and does writing an element through the link update the intended namespace variable?

This case informs name resolution and diagnostics in the LSP, compiler and VM; it does not establish behavior in other environments.

## Assumptions

Recorded case scope: Complete original source in fresh stock child interpreter/process; source is exact, wrapper/isolation channel must still be retained.

Other assumptions were not separately recorded. Connection identifiers were redacted in the retained invocation.

## Script

Fixture: [bigip-stock-8.4-3](fixtures/bigip-stock-8.4-3.md)

- run: `nh20261012stock2`
- case: `keyed-variable-link`

### nh_operation

```tcl-visible
namespace eval ::nh {
    set {state.a b(value)} original
    proc change {name} {
        upvar ::nh::$name state
        set state(value) changed
    }
}
::nh::change {state.a b}
set {::nh::state.a b(value)}

```

## Results

### BIG-IP stock-8.4

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: SSH to /usr/bin/tclsh8.4, script on standard input.
- Exit status: 0
- Standard output: decoded below.

- Standard error:

```text-visible
** WARNING: connection is not using a post-quantum key exchange algorithm.⟦CRLF⟧
** This session may be vulnerable to "store now, decrypt later" attacks.⟦CRLF⟧
** The server may need to be upgraded. See https://openssh.com/pq.html⟦CRLF⟧
```

The exit status and streams are from the SSH invocation; standard error can include SSH transport messages as well as remote errors. The entries below decode complete NHO result payloads from standard output.

- setup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
⟦EOF⟧
```

- lookup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
changed⟦EOF⟧
```
