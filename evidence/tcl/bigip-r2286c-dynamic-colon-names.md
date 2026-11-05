# BIG-IP original runtime: dynamic colon names

## Question

Do qualified variable references with two versus three consecutive colons resolve to the same value in this namespace? This constrains qualified-name parsing in the LSP, compiler and VM.

## Assumptions

The installed HTTP_REQUEST wrapper evaluated this exact script under catch. Repeated requests can share interpreter state, so later results need not match earlier results; counts are captured log lines, not proof of isolated invocations. The wrapper's errorCode field can reflect existing interpreter state. Results are limited to this context and inputs.

## Script

```tcl-visible
namespace eval ::__tcl_lsp_probe_2286_r2286_20261006c {variable x 7}
list [set ::__tcl_lsp_probe_2286_r2286_20261006c:::x] [set ::__tcl_lsp_probe_2286_r2286_20261006c::x]⟦EOF⟧
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
7 7⟦EOF⟧
```

The wrapper recorded Tcl completion code 0 in 34 log lines. Its recorded errorCode field was:

```text-visible
UNAVAILABLE⟦EOF⟧
```

### TMM unit 1, captured result 2; TMM unit 2, captured result 3

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.
- Recorded input channel: HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
7 7⟦EOF⟧
```

The wrapper recorded Tcl completion code 0 in 32 log lines. Its recorded errorCode field was:

```text-visible
UNAVAILABLE⟦EOF⟧
```

### TMM unit 3, captured result 4

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.
- Recorded input channel: HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
7 7⟦EOF⟧
```

The wrapper recorded Tcl completion code 0 in 32 log lines. Its recorded errorCode field was:

```text-visible
UNAVAILABLE⟦EOF⟧
```
