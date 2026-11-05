# BIG-IP original runtime: dynamic rename epoch

## Question

Does a computed command name resolve to a replacement after the original command is renamed, and can the moved command still be invoked? This constrains binding invalidation in the LSP, compiler and VM.

## Assumptions

The installed HTTP_REQUEST wrapper evaluated this exact script under catch. Repeated requests can share interpreter state, so later results need not match earlier results; counts are captured log lines, not proof of isolated invocations. The wrapper's errorCode field can reflect existing interpreter state. Results are limited to this context and inputs.

## Script

```tcl-visible
proc __tcl_lsp_probe_2286_r2286_20261006c_one {} {return OLD}
set c __tcl_lsp_probe_2286_r2286_20261006c_one
set before [eval $c]
rename __tcl_lsp_probe_2286_r2286_20261006c_one __tcl_lsp_probe_2286_r2286_20261006c_moved
proc __tcl_lsp_probe_2286_r2286_20261006c_one {} {return NEW}
list $before [eval $c] [__tcl_lsp_probe_2286_r2286_20261006c_moved]⟦EOF⟧
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
OLD NEW OLD⟦EOF⟧
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
can't rename to "__tcl_lsp_probe_2286_r2286_20261006c_moved": command already exists⟦EOF⟧
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
OLD NEW OLD⟦EOF⟧
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
can't rename to "__tcl_lsp_probe_2286_r2286_20261006c_moved": command already exists⟦EOF⟧
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
can't rename to "__tcl_lsp_probe_2286_r2286_20261006c_moved": command already exists⟦EOF⟧
```

The wrapper recorded Tcl completion code 1 in 31 log lines. Its recorded errorCode field was:

```text-visible
UNAVAILABLE⟦EOF⟧
```
