# BIG-IP original runtime: dynamic namespace import

## Question

Can a consumer import and call an exported namespace command, and what happens on repeated requests? This constrains imported command bindings in the LSP, compiler and VM.

## Assumptions

The installed HTTP_REQUEST wrapper evaluated this exact script under catch. Repeated requests can share interpreter state, so later results need not match earlier results; counts are captured log lines, not proof of isolated invocations. The wrapper's errorCode field can reflect existing interpreter state. Results are limited to this context and inputs.

## Script

```tcl-visible
namespace eval ::__tcl_lsp_probe_2286_r2286_20261006c {proc p {} {return EXPORTED}; namespace export p}
namespace eval ::__tcl_lsp_probe_2286_r2286_20261006c_consumer {namespace import ::__tcl_lsp_probe_2286_r2286_20261006c::p; p}⟦EOF⟧
```

## Results

### TMM unit 0, captured result 1

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.
- Recorded input channel: HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
EXPORTED⟦EOF⟧
```

The wrapper recorded Tcl completion code 0 in 2 log lines. Its recorded errorCode field was:

```text-visible
UNAVAILABLE⟦EOF⟧
```

### TMM unit 0, captured result 2

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.
- Recorded input channel: HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
can't import command "p": already exists⟦EOF⟧
```

The wrapper recorded Tcl completion code 1 in 32 log lines. Its recorded errorCode field was:

```text-visible
UNAVAILABLE⟦EOF⟧
```

### TMM unit 1, captured result 3; TMM unit 2, captured result 5; TMM unit 3, captured result 7

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.
- Recorded input channel: HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
EXPORTED⟦EOF⟧
```

The wrapper recorded Tcl completion code 0 in 1 log lines. Its recorded errorCode field was:

```text-visible
UNAVAILABLE⟦EOF⟧
```

### TMM unit 1, captured result 4; TMM unit 2, captured result 6

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.
- Recorded input channel: HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
can't import command "p": already exists⟦EOF⟧
```

The wrapper recorded Tcl completion code 1 in 31 log lines. Its recorded errorCode field was:

```text-visible
UNAVAILABLE⟦EOF⟧
```

### TMM unit 3, captured result 8

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.
- Recorded input channel: HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
can't import command "p": already exists⟦EOF⟧
```

The wrapper recorded Tcl completion code 1 in 31 log lines. Its recorded errorCode field was:

```text-visible
UNAVAILABLE⟦EOF⟧
```
