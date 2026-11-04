# BIG-IP variable lifecycle: active-element

## Question

When an array-element read trace unsets and recreates its element during lappend, which value does the active operation use and which callbacks survive? This constrains array-element lifetime in the compiler and VM.

## Assumptions

The payload was evaluated under catch in an HTTP_REQUEST event, sequentially with 60 other cases in the same event frame. Earlier cases can affect later cases, so the results do not assume a fresh interpreter or unset variables. The script's own nested catch and hex encoding, where present, remain part of the result. The observations are limited to the recorded context and inputs. The immediately preceding case was [active-scalar](bigip-r2286vl1-active-scalar.md).

## Script

```tcl-visible
set events {}; set a(k) OLD
proc watch {n k op} {upvar 1 events events; lappend events $op; if {$op eq "read"} {uplevel 1 {unset a(k); set a(k) NEW}}}
trace add variable a(k) {read unset} watch
set h lappend
set answer [$h a(k) EXTRA]
set summary [list $answer [set a(k)] $events [trace info variable a(k)]]
set summary
```

## Results

### TMM 0:0, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:0, HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{NEW EXTRA} {NEW EXTRA} {read unset} {}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The response from source port 42103 included this case in the sequential control matrix.

### TMM 0:1, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:1, HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{NEW EXTRA} {NEW EXTRA} {read unset} {}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The response from source port 42102 included this case in the sequential control matrix.

### TMM 0:2, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:2, HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{NEW EXTRA} {NEW EXTRA} {read unset} {}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The response from source port 42101 included this case in the sequential control matrix.

### TMM 0:3, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:3, HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{NEW EXTRA} {NEW EXTRA} {read unset} {}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The response from source port 42100 included this case in the sequential control matrix.
