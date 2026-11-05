# BIG-IP stock Tcl: namespace-captured-prefix

## Question

Does a namespace code prefix retain the defining namespace when evaluated elsewhere, including after a same-named global command is defined, and preserve its argument elements?

This case informs name resolution and diagnostics in the LSP, compiler and VM; it does not establish behavior in other environments.

## Assumptions

Recorded case scope: Complete original source in fresh stock child interpreter/process; source is exact, wrapper/isolation channel must still be retained.

Other assumptions were not separately recorded. Connection identifiers were redacted in the retained invocation.

## Script

Fixture: [bigip-stock-8.5-3](fixtures/bigip-stock-8.5-3.md)

- run: `nh20261012stock2`
- case: `namespace-captured-prefix`

### nh_operation

```tcl-visible
namespace eval ::nh {
    proc emit {value args} {list local $value $args}
    set prefix [namespace code [list emit {a b}]]
}
proc emit {args} {return global}
eval [linsert $::nh::prefix end {c d}]

```

## Results

### BIG-IP stock-8.5

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: SSH to /usr/bin/tclsh8.5, script on standard input.
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
local {a b} {{c d}}⟦EOF⟧
```
