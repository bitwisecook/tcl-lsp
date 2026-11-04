# BIG-IP array-element trace callback arguments

## Question

When a read trace unsets and recreates an array element during lappend, what root and index bytes are passed to the read and unset callbacks, and which value does lappend use? This constrains array-element identity and trace dispatch in the compiler and VM.

## Assumptions

The payload ran under catch in an HTTP_REQUEST wrapper. Callback names were encoded by the script using binary scan; the result below retains those encoded fields. The observation is limited to the recorded context and inputs.

## Script

```tcl-visible
set events {}
set a(k) OLD
proc __tcl_lsp_2286_vl1_argument_watch {n k op} {
    upvar 1 events events
    binary scan $n H* name_hex
    binary scan $k H* index_hex
    lappend events [list $op $name_hex $index_hex]
    if {$op eq "read"} {uplevel 1 {unset a(k); set a(k) NEW}}
}
trace add variable a(k) {read unset} __tcl_lsp_2286_vl1_argument_watch
set h lappend
set answer [$h a(k) EXTRA]
set summary [list $answer [set a(k)] $events [trace info variable a(k)]]
set summary
```

## Results

### TMM 0:3, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:3, HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{NEW EXTRA} {NEW EXTRA} {{read 61 6b} {unset 61 6b}} {}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The response was collected from source port 42100.

### TMM 0:2, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:2, HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{NEW EXTRA} {NEW EXTRA} {{read 61 6b} {unset 61 6b}} {}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The response was collected from source port 42101.

### TMM 0:1, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:1, HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{NEW EXTRA} {NEW EXTRA} {{read 61 6b} {unset 61 6b}} {}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The response was collected from source port 42102.

### TMM 0:0, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:0, HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{NEW EXTRA} {NEW EXTRA} {{read 61 6b} {unset 61 6b}} {}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The response was collected from source port 42103.
