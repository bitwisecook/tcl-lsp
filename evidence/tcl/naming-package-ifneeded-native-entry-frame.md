# Tcl execution evidence

## Question

> A loader registered inside namespace Registrar is required from procedure Caller::run, which has a localSentinel variable. An analyser could wrongly give the deferred callback either the registration namespace or the require caller frame. Family-specific scheduling must be distinguished from the authored body role, and a Jim registration rejection cannot answer an entered callback frame.

> On the fixed original ASCII source-file probe, what namespace, info level and caller-local visibility does package ifneeded use when package require invokes the loader from Caller::run, and does current Jim accept the registration?

## Assumptions

Assumptions were not separately recorded in the retained capture.

Recorded scope and limits:

> One complete fixed ASCII script in a fresh native shell process, with recorded shell argument vectors, exact byte streams. Actual info patchlevel is printed before registration. No counted object-vector, native compilation admission, opaque/zero names, variable links, multiple deferred loaders or appliance execution is measured.

## Script

```tcl-visible
puts "VERSION|[info patchlevel]"
set ::observed {}
namespace eval Registrar {
    package ifneeded R2286FutureFrame 1.0 {
        lappend ::observed [list [namespace current] [info level] [info exists localSentinel]]
        package provide R2286FutureFrame 1.0
    }
}
namespace eval Caller {
    proc run {} {
        set localSentinel local
        package require R2286FutureFrame
    }
}
set code [catch {Caller::run} result]
puts [list RESULT $code $result $::observed]
```

## Results

### tcl8.4

- Recorded version: 8.4.20
- Recorded input channel: Fixed ASCII script-file argument in a fresh native shell process
- Exit status: 0
- Standard output:

```text-visible
VERSION|8.4.20
RESULT 0 1.0 {{:: 0 0}}
```

- Standard error: empty.

### tcl8.5

- Recorded version: 8.5.19
- Recorded input channel: Fixed ASCII script-file argument in a fresh native shell process
- Exit status: 0
- Standard output:

```text-visible
VERSION|8.5.19
RESULT 0 1.0 {{:: 0 0}}
```

- Standard error: empty.

### tcl8.6

- Recorded version: 8.6.18
- Recorded input channel: Fixed ASCII script-file argument in a fresh native shell process
- Exit status: 0
- Standard output:

```text-visible
VERSION|8.6.18
RESULT 0 1.0 {{:: 0 0}}
```

- Standard error: empty.

### tcl9.0

- Recorded version: 9.0.4
- Recorded input channel: Fixed ASCII script-file argument in a fresh native shell process
- Exit status: 0
- Standard output:

```text-visible
VERSION|9.0.4
RESULT 0 1.0 {{:: 0 0}}
```

- Standard error: empty.

### tcl9.1

- Recorded version: 9.1.0
- Recorded input channel: Fixed ASCII script-file argument in a fresh native shell process
- Exit status: 0
- Standard output:

```text-visible
VERSION|9.1.0
RESULT 0 1.0 {{:: 0 0}}
```

- Standard error: empty.

### jim

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: Fixed ASCII script-file argument in a fresh native shell process
- Exit status: 1
- Standard output:

```text-visible
VERSION|0.84-9-g5bac7c9
```

- Standard error:

```text-visible
/workspace/.proofs/captured-package-future-frame121/probe.tcl:4: Error: package, unknown command "ifneeded": should be forget, names, provide, require
Traceback (most recent call last):
  File "/workspace/.proofs/captured-package-future-frame121/probe.tcl", line 4
    package ifneeded R2286FutureFrame 1.0 {...
```
