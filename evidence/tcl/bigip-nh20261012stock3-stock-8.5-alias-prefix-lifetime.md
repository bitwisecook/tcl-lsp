# BIG-IP stock Tcl: alias-prefix-lifetime

## Question

After an interpreter alias with a fixed multiword argument is renamed, what does querying the old alias name return, and how does the renamed command apply its retained prefix to a new argument?

This case informs name resolution and diagnostics in the LSP, compiler and VM; it does not establish behavior in other environments.

## Assumptions

Recorded case scope: Original authored names and concatenated source bytes; setup/final command split in a fresh stock process. Process destruction supplies interpreter isolation; no in-process cleanup. This is a wrapper/frame/catch-boundary adaptation, not an exact portable-input answer.

Other assumptions were not separately recorded. Connection identifiers were redacted in the retained invocation.

## Script

Fixture: [bigip-stock-8.5-1](fixtures/bigip-stock-8.5-1.md)

- run: `nh20261012stock3`
- case: `alias-prefix-lifetime`

### nh_setup

```tcl-visible
namespace eval ::nh {proc emit {fixed value} {list $fixed $value}}
interp alias {} ::nh::local {} ::nh::emit {a b}
rename ::nh::local ::nh::renamed
set description [interp alias {} ::nh::local]
```

### nh_operation

```tcl-visible
list $description [::nh::renamed {c d}]

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
::nh::emit {a b}⟦EOF⟧
```

- setup, completion 0, returnoptions (utf-8; unit none, invocation once):

```text-visible
-code 0 -level 0⟦EOF⟧
```

- lookup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
{::nh::emit {a b}} {{a b} {c d}}⟦EOF⟧
```

- lookup, completion 0, returnoptions (utf-8; unit none, invocation once):

```text-visible
-code 0 -level 0⟦EOF⟧
```
