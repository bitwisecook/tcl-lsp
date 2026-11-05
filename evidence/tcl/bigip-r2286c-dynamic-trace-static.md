# BIG-IP original runtime: dynamic trace static

## Question

What callbacks occur when a static variable is read, unset and recreated after a legacy trace is installed? This constrains variable lifetime and trace dispatch in the compiler and VM.

## Assumptions

The installed HTTP_REQUEST wrapper evaluated this exact script under catch. Repeated requests can share interpreter state, so later results need not match earlier results; counts are captured log lines, not proof of isolated invocations. The wrapper's errorCode field can reflect existing interpreter state. Results are limited to this context and inputs.

## Script

```tcl-visible
set static::__tcl_lsp_probe_2286_r2286_20261006c_events {}
proc __tcl_lsp_probe_2286_r2286_20261006c_trace {n i op} {lappend static::__tcl_lsp_probe_2286_r2286_20261006c_events [list $n $i $op]}
set static::__tcl_lsp_probe_2286_r2286_20261006c_x 1
trace variable static::__tcl_lsp_probe_2286_r2286_20261006c_x rwu __tcl_lsp_probe_2286_r2286_20261006c_trace
set a [set static::__tcl_lsp_probe_2286_r2286_20261006c_x]
unset static::__tcl_lsp_probe_2286_r2286_20261006c_x
set static::__tcl_lsp_probe_2286_r2286_20261006c_x 3
list $a $static::__tcl_lsp_probe_2286_r2286_20261006c_events⟦EOF⟧
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
1 {{static::__tcl_lsp_probe_2286_r2286_20261006c_x {} r} {static::__tcl_lsp_probe_2286_r2286_20261006c_x {} u}}⟦EOF⟧
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
1 {{static::__tcl_lsp_probe_2286_r2286_20261006c_x {} r} {static::__tcl_lsp_probe_2286_r2286_20261006c_x {} u}}⟦EOF⟧
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
1 {{static::__tcl_lsp_probe_2286_r2286_20261006c_x {} r} {static::__tcl_lsp_probe_2286_r2286_20261006c_x {} u}}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0 in 32 log lines. Its recorded errorCode field was:

```text-visible
UNAVAILABLE⟦EOF⟧
```
