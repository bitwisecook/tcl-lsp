# BIG-IP stock Tcl: frame-wrapper-near-miss

## Question

When the same forwarding operation uses eval rather than uplevel 1, which caller frame does upvar 1 reach, and is the original caller's spaced variable changed?

This case informs name resolution and diagnostics in the LSP, compiler and VM; it does not establish behavior in other environments.

## Assumptions

Recorded case scope: Complete original source in fresh stock child interpreter/process; source is exact, wrapper/isolation channel must still be retained.

Other assumptions were not separately recorded. Connection identifiers were redacted in the retained invocation.

## Script

Fixture: [bigip-stock-8.4-3](fixtures/bigip-stock-8.4-3.md)

- run: `nh20261012stock2`
- case: `frame-wrapper-near-miss`

### nh_operation

```tcl-visible
proc forward {prefix} {eval $prefix}
proc change {name} {upvar 1 $name cell; set cell changed}
proc outer {} {
    set {a b} original
    forward [list change {a b}]
    set {a b}
}
outer

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
original⟦EOF⟧
```
