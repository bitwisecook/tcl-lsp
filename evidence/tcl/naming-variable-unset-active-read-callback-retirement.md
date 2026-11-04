# Tcl execution evidence

## Question

> An active scalar read callback unsets and recreates the same written scalar. Its old unset observers, old read registration and later read result have separate purposes; a trace setup rejection cannot answer any entered callback behavior.

> For the exact original ASCII scalar probe, does unsetting during an active read invoke the old unset callback, retire the old registrations, and let the later read observe the recreated value without rerunning the old read callback?

## Assumptions

Assumptions were not separately recorded in the retained capture.

Recorded scope and limits:

> One fixed scalar script per actual C8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 and current Jim fresh CLI process. Only the recorded source setup/completion/result sequence is measured. No physical captured receiver, pointer/header epoch, compiler admission, array-element identity, generic increment/container claim or BIG-IP answer follows. Linked C port controls replay the identical complete original source and compare the full observation output after the independently identified PATCHLEVEL line. Linked Jim controls execute only the identical original source prefix through trace SETUP and compare the stored setup code/message with the original SETUP bytes; the unrelated binary formatter and callback body are not replayed. The original complete native source and stdout remain immutable.

## Script

```tcl-visible
if {[catch {info patchlevel} probeVersion]} {
    set probeVersion unavailable
}
puts "PATCHLEVEL $probeVersion"
set ::probeLog {}
proc probeTrace {name element operation} {
    lappend ::probeLog $operation
    if {[string equal $operation read]} {
        unset ::probeValue
        set ::probeValue 7
    }
}
set ::probeValue 1
set probeSetupCode [catch {
    trace add variable ::probeValue {read unset} probeTrace
} probeSetupResult]
binary scan $probeSetupResult H* probeSetupHex
puts "SETUP $probeSetupCode $probeSetupHex"
if {!$probeSetupCode} {
    set probeCode [catch {
        set probeRead $::probeValue
        list $probeRead $::probeValue $::probeLog [trace info variable ::probeValue]
    } probeResult]
    binary scan $probeResult H* probeResultHex
    puts "OBSERVATION $probeCode $probeResultHex"
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
OBSERVATION 0 372037207b7265616420756e7365747d207b7d
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
OBSERVATION 0 372037207b7265616420756e7365747d207b7d
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
OBSERVATION 0 372037207b7265616420756e7365747d207b7d
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
OBSERVATION 0 372037207b7265616420756e7365747d207b7d
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
OBSERVATION 0 372037207b7265616420756e7365747d207b7d
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
