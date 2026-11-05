# BIG-IP stock Tcl: ensemble-prefix-map

## Question

Does an ensemble subcommand mapping preserve the elements of a list command prefix, including a fixed argument containing a space, when passing a new argument?

This case informs name resolution and diagnostics in the LSP, compiler and VM; it does not establish behavior in other environments.

## Assumptions

Recorded case scope: Original authored names and concatenated source bytes; setup/final command split in a fresh stock process. Process destruction supplies interpreter isolation; no in-process cleanup. This is a wrapper/frame/catch-boundary adaptation, not an exact portable-input answer.

The intended operation was not reached; this capture records the earlier outcome.

Other assumptions were not separately recorded. Connection identifiers were redacted in the retained invocation.

## Script

Fixture: [bigip-stock-8.4-1](fixtures/bigip-stock-8.4-1.md)

- run: `nh20261012pat3`
- case: `ensemble-prefix-map`

### nh_setup

```tcl-visible
namespace eval ::nh {
    proc emit {fixed value} {list $fixed $value}
    namespace ensemble create -command facade -map [list send [list ::nh::emit {a b}]]
}
```

### nh_operation

```tcl-visible
::nh::facade send {c d}

```

## Results

### BIG-IP stock-8.4

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: SSH to /usr/bin/tclsh8.4, script on standard input.
- Exit status: 0
- Standard output: decoded below.

- Standard error: empty.

The exit status and streams are from the SSH invocation; standard error can include SSH transport messages as well as remote errors. The entries below decode complete NHO result payloads from standard output.

- setup, completion 1, result (utf-8; unit none, invocation once):

```text-visible
bad option "ensemble": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which⟦EOF⟧
```

- setup, completion 1, metadata (utf-8; unit none, invocation once):

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- setup, completion 1, errorcode (utf-8; unit none, invocation once):

```text-visible
NONE⟦EOF⟧
```

- setup, completion 1, errorinfo (utf-8; unit none, invocation once):

```text-visible
bad option "ensemble": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which
    while executing
"namespace ensemble create -command facade -map [list send [list ::nh::emit {a b}]]"
    (in namespace eval "::nh" script line 3)
    invoked from within
"namespace eval ::nh {
    proc emit {fixed value} {list $fixed $value}
    namespace ensemble create -command facade -map [list send [list ::nh::emit ..."
    ("eval" body line 1)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```
