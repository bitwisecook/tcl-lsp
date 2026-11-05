# BIG-IP stock Tcl: factory-body-near-miss

## Question

When a loop defines procedures using a braced body containing the loop variable, is that variable substituted during definition or looked up when the generated procedure executes?

This case informs name resolution and diagnostics in the LSP, compiler and VM; it does not establish behavior in other environments.

## Assumptions

Recorded case scope: Complete original source in fresh stock child interpreter/process; source is exact, wrapper/isolation channel must still be retained.

Other assumptions were not separately recorded. Connection identifiers were redacted in the retained invocation.

## Script

Fixture: [bigip-stock-8.5-3](fixtures/bigip-stock-8.5-3.md)

- run: `nh20261012stock2`
- case: `factory-body-near-miss`

### nh_operation

```tcl-visible
namespace eval ::nh {
    foreach name {red blue} {proc $name {} {return $name}}
}
::nh::red

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

- lookup, completion 1, result (utf-8; unit none, invocation once):

```text-visible
can't read "name": no such variable⟦EOF⟧
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
can't read "name": no such variable
    while executing
"return $name"
    (procedure "::nh::red" line 1)
    invoked from within
"::nh::red"
    ("eval" body line 4)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```
