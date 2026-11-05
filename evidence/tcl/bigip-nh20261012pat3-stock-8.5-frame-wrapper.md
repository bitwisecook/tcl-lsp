# BIG-IP stock Tcl: frame-wrapper

## Question

Does a forwarding procedure using uplevel 1 let a subsequent upvar 1 reach a spaced variable name in the original caller's frame?

This case informs name resolution and diagnostics in the LSP, compiler and VM; it does not establish behavior in other environments.

## Assumptions

Recorded case scope: Original authored names and concatenated source bytes; setup/final command split in a fresh stock process. Process destruction supplies interpreter isolation; no in-process cleanup. This is a wrapper/frame/catch-boundary adaptation, not an exact portable-input answer.

Other assumptions were not separately recorded. Connection identifiers were redacted in the retained invocation.

## Script

Fixture: [bigip-stock-8.5-1](fixtures/bigip-stock-8.5-1.md)

- run: `nh20261012pat3`
- case: `frame-wrapper`

### nh_setup

```tcl-visible
proc forward {prefix} {uplevel 1 $prefix}
proc change {name} {upvar 1 $name cell; set cell changed}
proc outer {} {
    set {a b} original
    forward [list change {a b}]
    set {a b}
}
```

### nh_operation

```tcl-visible
outer

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
⟦EOF⟧
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
