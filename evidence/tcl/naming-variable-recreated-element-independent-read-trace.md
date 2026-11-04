# Tcl execution evidence

## Question

> An old array-element read callback destroys the entire array, recreates the same written member and installs a new read observer before the old callback returns. The new read observation and the old pending increment completion cannot be inferred from matching key text.

> For the exact original ASCII array probe, can the newly created same-key member run its new read observer before the old callback returns, and what completion does the old captured increment report in each measured C release?

## Assumptions

Assumptions were not separately recorded in the retained capture.

Recorded scope and limits:

> One fixed ASCII source-file CLI script in each of five actual C releases and current Jim. The new read, callback log, old increment error and final value are measured as one guest result. No native pointer/header, compiled receiver recipe, dictionary key allocation identity, generic physical stale-handle claim, scalar-retirement generalization or appliance behavior follows. Linked C port controls replay the identical complete original source and compare the full observation output after the independently identified PATCHLEVEL line. Linked Jim controls execute only the identical original source prefix through trace SETUP and compare the stored setup code/message with the original SETUP bytes; the unrelated binary formatter and callback body are not replayed. The original complete native source and stdout remain immutable.

## Script

```tcl-visible
if {[catch {info patchlevel} probeVersion]} {
    set probeVersion unavailable
}
puts "PATCHLEVEL $probeVersion"
set ::a(k) 10
set ::probeLog {}
proc probeNew args {
    lappend ::probeLog READ
}
proc probeOld args {
    unset ::a
    set ::a(k) 100
    trace add variable ::a(k) read probeNew
    set ::probeSeen [set ::a(k)]
}
set probeSetupCode [catch {
    trace add variable ::a(k) read probeOld
} probeSetupResult]
binary scan $probeSetupResult H* probeSetupHex
puts "SETUP $probeSetupCode $probeSetupHex"
if {!$probeSetupCode} {
    set probeCode [catch {incr ::a(k)} probeResult]
    trace remove variable ::a(k) read probeNew
    set probeObservation [list $probeCode $probeResult $::probeSeen $::probeLog $::a(k)]
    binary scan $probeObservation H* probeHex
    puts "OBSERVATION $probeHex"
}
```

## Results

### tcl8.4

- Recorded version: 8.4.20
- Recorded input channel: One fixed complete ASCII LF source-file CLI argument in a fresh native shell process.
- Exit status: 0
- Standard output:

```text-visible
PATCHLEVEL 8.4.20
SETUP 0⟦SPACE⟧
OBSERVATION 31207b63616e2774207265616420223a3a61286b29223a206e6f207375636820656c656d656e7420696e2061727261797d20313030205245414420313030
```

- Standard error: empty.

### tcl8.5

- Recorded version: 8.5.19
- Recorded input channel: One fixed complete ASCII LF source-file CLI argument in a fresh native shell process.
- Exit status: 0
- Standard output:

```text-visible
PATCHLEVEL 8.5.19
SETUP 0⟦SPACE⟧
OBSERVATION 31207b63616e27742073657420223a3a61286b29223a2075707661722072656665727320746f20656c656d656e7420696e2064656c657465642061727261797d20313030205245414420313030
```

- Standard error: empty.

### tcl8.6

- Recorded version: 8.6.18
- Recorded input channel: One fixed complete ASCII LF source-file CLI argument in a fresh native shell process.
- Exit status: 0
- Standard output:

```text-visible
PATCHLEVEL 8.6.18
SETUP 0⟦SPACE⟧
OBSERVATION 31207b63616e27742073657420223a3a61286b29223a2075707661722072656665727320746f20656c656d656e7420696e2064656c657465642061727261797d20313030205245414420313030
```

- Standard error: empty.

### tcl9.0

- Recorded version: 9.0.4
- Recorded input channel: One fixed complete ASCII LF source-file CLI argument in a fresh native shell process.
- Exit status: 0
- Standard output:

```text-visible
PATCHLEVEL 9.0.4
SETUP 0⟦SPACE⟧
OBSERVATION 31207b63616e27742073657420223a3a61286b29223a2075707661722072656665727320746f20656c656d656e7420696e2064656c657465642061727261797d20313030205245414420313030
```

- Standard error: empty.

### tcl9.1

- Recorded version: 9.1.0
- Recorded input channel: One fixed complete ASCII LF source-file CLI argument in a fresh native shell process.
- Exit status: 0
- Standard output:

```text-visible
PATCHLEVEL 9.1.0
SETUP 0⟦SPACE⟧
OBSERVATION 31207b63616e27742073657420223a3a61286b29223a2075707661722072656665727320746f20656c656d656e7420696e2064656c657465642061727261797d20313030205245414420313030
```

- Standard error: empty.

### jim

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: One fixed complete ASCII LF source-file CLI argument in a fresh native shell process.
- Exit status: 0
- Standard output:

```text-visible
PATCHLEVEL 0.84-9-g5bac7c9
SETUP 1 696e76616c696420636f6d6d616e64206e616d652022747261636522
```

- Standard error: empty.
