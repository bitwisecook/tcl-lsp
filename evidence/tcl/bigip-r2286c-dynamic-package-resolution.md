# BIG-IP original runtime: dynamic package resolution

## Question

Does package ifneeded register a script that package require executes to provide the requested version? This constrains package discovery and evaluation modeling in the LSP and runtime.

## Assumptions

The installed HTTP_REQUEST wrapper evaluated this exact script under catch. Repeated requests can share interpreter state, so later results need not match earlier results; counts are captured log lines, not proof of isolated invocations. The wrapper's errorCode field can reflect existing interpreter state. Results are limited to this context and inputs.

## Script

```tcl-visible
set script "package provide __tcl_lsp_probe_2286_r2286_20261006c_package 1.0"
package ifneeded __tcl_lsp_probe_2286_r2286_20261006c_package 1.0 $script
list [package require __tcl_lsp_probe_2286_r2286_20261006c_package 1.0] [package provide __tcl_lsp_probe_2286_r2286_20261006c_package]⟦EOF⟧
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
1.0 1.0⟦EOF⟧
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
1.0 1.0⟦EOF⟧
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
1.0 1.0⟦EOF⟧
```

The wrapper recorded Tcl completion code 0 in 32 log lines. Its recorded errorCode field was:

```text-visible
UNAVAILABLE⟦EOF⟧
```
