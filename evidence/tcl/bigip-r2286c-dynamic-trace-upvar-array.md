# BIG-IP original runtime: dynamic trace upvar array

## Question

What trace callbacks and values result when an array element is read through upvar #0 across repeated requests? This constrains array identity, links and trace dispatch in the compiler and VM.

## Assumptions

The installed HTTP_REQUEST wrapper evaluated this exact script under catch. Repeated requests can share interpreter state, so later results need not match earlier results; counts are captured log lines, not proof of isolated invocations. The wrapper's errorCode field can reflect existing interpreter state. Results are limited to this context and inputs.

## Script

```tcl-visible
set ::__tcl_lsp_probe_2286_r2286_20261006c_events {}
proc __tcl_lsp_probe_2286_r2286_20261006c_trace {n i op} {lappend ::__tcl_lsp_probe_2286_r2286_20261006c_events [list $n $i $op]}
set ::__tcl_lsp_probe_2286_r2286_20261006c_arr(k) 1
trace variable ::__tcl_lsp_probe_2286_r2286_20261006c_arr rwu __tcl_lsp_probe_2286_r2286_20261006c_trace
proc __tcl_lsp_probe_2286_r2286_20261006c_read {} {upvar #0 ::__tcl_lsp_probe_2286_r2286_20261006c_arr(k) v; return $v}
list [__tcl_lsp_probe_2286_r2286_20261006c_read] $::__tcl_lsp_probe_2286_r2286_20261006c_events⟦EOF⟧
```

## Results

### TMM unit 0, captured result 1; TMM unit 1, captured result 9; TMM unit 2, captured result 17; TMM unit 3, captured result 25

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.
- Recorded input channel: HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
1 {{::__tcl_lsp_probe_2286_r2286_20261006c_arr k w} {::__tcl_lsp_probe_2286_r2286_20261006c_arr k w} {::__tcl_lsp_probe_2286_r2286_20261006c_arr k w} {::__tcl_lsp_probe_2286_r2286_20261006c_arr k w} {::__tcl_lsp_probe_2286_r2286_20261006c_arr k w} {::__tcl_lsp_probe_2286_r2286_20261006c_arr k w} {::__tcl_lsp_probe_2286_r2286_20261006c_arr k w}}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0 in 1 log lines. Its recorded errorCode field was:

```text-visible
UNAVAILABLE⟦EOF⟧
```

### TMM unit 0, captured result 2; TMM unit 1, captured result 10; TMM unit 2, captured result 18; TMM unit 3, captured result 26

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.
- Recorded input channel: HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
1 {{::__tcl_lsp_probe_2286_r2286_20261006c_arr k w} {::__tcl_lsp_probe_2286_r2286_20261006c_arr k w} {::__tcl_lsp_probe_2286_r2286_20261006c_arr k w} {::__tcl_lsp_probe_2286_r2286_20261006c_arr k w} {::__tcl_lsp_probe_2286_r2286_20261006c_arr k w} {::__tcl_lsp_probe_2286_r2286_20261006c_arr k w}}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0 in 1 log lines. Its recorded errorCode field was:

```text-visible
UNAVAILABLE⟦EOF⟧
```

### TMM unit 0, captured result 3; TMM unit 1, captured result 11; TMM unit 2, captured result 19; TMM unit 3, captured result 27

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.
- Recorded input channel: HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
1 {{::__tcl_lsp_probe_2286_r2286_20261006c_arr k w} {::__tcl_lsp_probe_2286_r2286_20261006c_arr k w} {::__tcl_lsp_probe_2286_r2286_20261006c_arr k w} {::__tcl_lsp_probe_2286_r2286_20261006c_arr k w} {::__tcl_lsp_probe_2286_r2286_20261006c_arr k w}}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0 in 1 log lines. Its recorded errorCode field was:

```text-visible
UNAVAILABLE⟦EOF⟧
```

### TMM unit 0, captured result 4; TMM unit 1, captured result 12; TMM unit 2, captured result 20; TMM unit 3, captured result 28

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.
- Recorded input channel: HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
1 {{::__tcl_lsp_probe_2286_r2286_20261006c_arr k w} {::__tcl_lsp_probe_2286_r2286_20261006c_arr k w} {::__tcl_lsp_probe_2286_r2286_20261006c_arr k w} {::__tcl_lsp_probe_2286_r2286_20261006c_arr k w}}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0 in 1 log lines. Its recorded errorCode field was:

```text-visible
UNAVAILABLE⟦EOF⟧
```

### TMM unit 0, captured result 5; TMM unit 1, captured result 13; TMM unit 2, captured result 21; TMM unit 3, captured result 29

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.
- Recorded input channel: HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
1 {{::__tcl_lsp_probe_2286_r2286_20261006c_arr k w} {::__tcl_lsp_probe_2286_r2286_20261006c_arr k w} {::__tcl_lsp_probe_2286_r2286_20261006c_arr k w}}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0 in 1 log lines. Its recorded errorCode field was:

```text-visible
UNAVAILABLE⟦EOF⟧
```

### TMM unit 0, captured result 6; TMM unit 1, captured result 14; TMM unit 2, captured result 22; TMM unit 3, captured result 30

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.
- Recorded input channel: HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
1 {{::__tcl_lsp_probe_2286_r2286_20261006c_arr k w} {::__tcl_lsp_probe_2286_r2286_20261006c_arr k w}}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0 in 1 log lines. Its recorded errorCode field was:

```text-visible
UNAVAILABLE⟦EOF⟧
```

### TMM unit 0, captured result 7

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.
- Recorded input channel: HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
1 {{::__tcl_lsp_probe_2286_r2286_20261006c_arr k w}}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0 in 2 log lines. Its recorded errorCode field was:

```text-visible
UNAVAILABLE⟦EOF⟧
```

### TMM unit 0, captured result 8

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.
- Recorded input channel: HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
1 {}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0 in 2 log lines. Its recorded errorCode field was:

```text-visible
UNAVAILABLE⟦EOF⟧
```

### TMM unit 1, captured result 15; TMM unit 2, captured result 23; TMM unit 3, captured result 31

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.
- Recorded input channel: HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
1 {{::__tcl_lsp_probe_2286_r2286_20261006c_arr k w}}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0 in 1 log lines. Its recorded errorCode field was:

```text-visible
UNAVAILABLE⟦EOF⟧
```

### TMM unit 1, captured result 16; TMM unit 2, captured result 24; TMM unit 3, captured result 32

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.
- Recorded input channel: HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
1 {}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0 in 1 log lines. Its recorded errorCode field was:

```text-visible
UNAVAILABLE⟦EOF⟧
```

### Truncated HTTP_REQUEST log fragment 1

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.
- Recorded input channel: HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
R2286|r2286_20261006c|dynamic_trace_upvar_array|unit=0|rc=0|result_hex=31207b7b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74⟦EOF⟧
```

This incomplete marker appeared in 23 log lines. The result and errorCode are truncated; the marker is shown verbatim and does not establish the missing bytes.

### Truncated HTTP_REQUEST log fragment 2

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.
- Recorded input channel: HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
R2286|r2286_20261006c|dynamic_trace_upvar_array|unit=0|rc=0|result_hex=31207b7b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d7d|errorcode_h⟦EOF⟧
```

This incomplete marker appeared in 1 log lines. The result and errorCode are truncated; the marker is shown verbatim and does not establish the missing bytes.

### Truncated HTTP_REQUEST log fragment 3

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.
- Recorded input channel: HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
R2286|r2286_20261006c|dynamic_trace_upvar_array|unit=1|rc=0|result_hex=31207b7b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f7⟦EOF⟧
```

This incomplete marker appeared in 23 log lines. The result and errorCode are truncated; the marker is shown verbatim and does not establish the missing bytes.

### Truncated HTTP_REQUEST log fragment 4

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.
- Recorded input channel: HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
R2286|r2286_20261006c|dynamic_trace_upvar_array|unit=1|rc=0|result_hex=31207b7b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d7d|errorcode_⟦EOF⟧
```

This incomplete marker appeared in 1 log lines. The result and errorCode are truncated; the marker is shown verbatim and does not establish the missing bytes.

### Truncated HTTP_REQUEST log fragment 5

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.
- Recorded input channel: HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
R2286|r2286_20261006c|dynamic_trace_upvar_array|unit=2|rc=0|result_hex=31207b7b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f7⟦EOF⟧
```

This incomplete marker appeared in 23 log lines. The result and errorCode are truncated; the marker is shown verbatim and does not establish the missing bytes.

### Truncated HTTP_REQUEST log fragment 6

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.
- Recorded input channel: HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
R2286|r2286_20261006c|dynamic_trace_upvar_array|unit=2|rc=0|result_hex=31207b7b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d7d|errorcode_⟦EOF⟧
```

This incomplete marker appeared in 1 log lines. The result and errorCode are truncated; the marker is shown verbatim and does not establish the missing bytes.

### Truncated HTTP_REQUEST log fragment 7

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.
- Recorded input channel: HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
R2286|r2286_20261006c|dynamic_trace_upvar_array|unit=3|rc=0|result_hex=31207b7b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f7⟦EOF⟧
```

This incomplete marker appeared in 23 log lines. The result and errorCode are truncated; the marker is shown verbatim and does not establish the missing bytes.

### Truncated HTTP_REQUEST log fragment 8

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.
- Recorded input channel: HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
R2286|r2286_20261006c|dynamic_trace_upvar_array|unit=3|rc=0|result_hex=31207b7b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d207b3a3a5f5f74636c5f6c73705f70726f62655f323238365f72323238365f3230323631303036635f617272206b20777d7d|errorcode_⟦EOF⟧
```

This incomplete marker appeared in 1 log lines. The result and errorCode are truncated; the marker is shown verbatim and does not establish the missing bytes.
