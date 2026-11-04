# Empty foreach keeps observer effects separate

## Question

Does a stock empty foreach avoid variable-write observations while preserving command-entry observer mutations and errors where those observer interfaces exist? This checks the observer exclusions required by #2441 source analysis, without treating an unavailable interface as observed behaviour.

## Assumptions

The script checks interface availability. C Tcl variable trace registration uses the modern form when accepted and the older form otherwise. Jim has no trace command in this build, so its variable observation is unavailable; the five C Tcl builds have no xtrace command, so their command-entry observation is unavailable. Jim xtrace callbacks run with tracing disabled by the engine. Additional startup assumptions: Not recorded. Each interpreter starts a fresh process. An earlier script used trace variable directly and produced errors on Tcl 9; its source and raw captures remain in the development receipt and are not these successful runs.

## Script

```tcl-visible
if {[llength [info commands trace]]} {
    set events 0
    proc writeObserver {name element operation} {incr ::events}
    if {[catch {trace add variable cell write writeObserver}]} {
        trace variable cell w writeObserver
    }
    foreach cell {} {error BODY}
    puts [list variable-observer-empty $events [info exists cell]]
    foreach cell {VALUE} {}
    puts [list variable-observer-nonempty $events $cell]
    catch {trace remove variable cell write writeObserver}
    catch {trace vdelete cell w writeObserver}
} else {
    puts [list variable-observer UNAVAILABLE]
}
if {[llength [info commands xtrace]]} {
    proc preserved {} {return ORIGINAL}
    proc commandObserver {type file line result command arguments} {
        if {$command eq "foreach"} {proc preserved {} {return OBSERVED}}
    }
    xtrace commandObserver
    foreach cell {} {error BODY}
    xtrace {}
    puts [list command-observer-empty [preserved] [info exists cell]]
    proc refusingObserver {type file line result command arguments} {
        if {$command eq "foreach"} {error OBSERVER_ERROR}
    }
    xtrace refusingObserver
    set code [catch {foreach cell {} {error BODY}} result]
    xtrace {}
    puts [list command-observer-error $code $result [info exists cell]]
} else {
    puts [list command-observer UNAVAILABLE]
}
```

## Results

### Tcl 8.4 8.4.20

- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh8.4` with Script bytes on standard input.
- Version observation: `puts [info patchlevel]` on standard input; exit 0.
- Exit status: 0.
- Standard output:

```text-visible
variable-observer-empty 0 0
variable-observer-nonempty 1 VALUE
command-observer UNAVAILABLE
```

- Standard error:

```text-visible
⟦EOF⟧
```

### Tcl 8.5 8.5.19

- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh8.5` with Script bytes on standard input.
- Version observation: `puts [info patchlevel]` on standard input; exit 0.
- Exit status: 0.
- Standard output:

```text-visible
variable-observer-empty 0 0
variable-observer-nonempty 1 VALUE
command-observer UNAVAILABLE
```

- Standard error:

```text-visible
⟦EOF⟧
```

### Tcl 8.6 8.6.18

- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh8.6` with Script bytes on standard input.
- Version observation: `puts [info patchlevel]` on standard input; exit 0.
- Exit status: 0.
- Standard output:

```text-visible
variable-observer-empty 0 0
variable-observer-nonempty 1 VALUE
command-observer UNAVAILABLE
```

- Standard error:

```text-visible
⟦EOF⟧
```

### Tcl 9.0 9.0.4

- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh9.0` with Script bytes on standard input.
- Version observation: `puts [info patchlevel]` on standard input; exit 0.
- Exit status: 0.
- Standard output:

```text-visible
variable-observer-empty 0 0
variable-observer-nonempty 1 VALUE
command-observer UNAVAILABLE
```

- Standard error:

```text-visible
⟦EOF⟧
```

### Tcl 9.1 9.1.0

- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh9.1` with Script bytes on standard input.
- Version observation: `puts [info patchlevel]` on standard input; exit 0.
- Exit status: 0.
- Standard output:

```text-visible
variable-observer-empty 0 0
variable-observer-nonempty 1 VALUE
command-observer UNAVAILABLE
```

- Standard error:

```text-visible
⟦EOF⟧
```

### JimTcl 0.84-10-g82fb4d0

- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/jimtcl/jimsh` with Script bytes on standard input.
- Version observation: `puts [info patchlevel]` on standard input; exit 0.
- Exit status: 0.
- Standard output:

```text-visible
variable-observer UNAVAILABLE
command-observer-empty OBSERVED 0
command-observer-error 1 OBSERVER_ERROR 0
```

- Standard error:

```text-visible
⟦EOF⟧
```
