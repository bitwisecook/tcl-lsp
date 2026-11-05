# BIG-IP stock Tcl: string-prefix-near-miss

## Question

What happens when a command name containing spaces is interpolated into an eval script string rather than preserved as a list command-prefix element?

This case informs name resolution and diagnostics in the LSP, compiler and VM; it does not establish behavior in other environments.

## Assumptions

Recorded case scope: Complete original source in fresh stock child interpreter/process; source is exact, wrapper/isolation channel must still be retained.

Other assumptions were not separately recorded. Connection identifiers were redacted in the retained invocation.

## Script

Fixture: [bigip-stock-8.4-3](fixtures/bigip-stock-8.4-3.md)

- run: `nh20261012pat1`
- case: `string-prefix-near-miss`

### nh_operation

```tcl-visible
namespace eval ::nh {}
proc {::nh::two words} {fixed value} {list $fixed $value}
set target {::nh::two words}
eval "$target {a b} {c d}"

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
    ("eval" body line 4)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```
