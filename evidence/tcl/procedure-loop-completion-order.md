# Procedure loop-completion conversion order

## Question

Does a procedure turn both direct `break`/`continue` and codes exposed by
`return -code` into errors, or does the latter reach a caller's loop? The
registry's ordered procedure-boundary conversions must preserve this distinction.

## Assumptions

Each source-built C Tcl wrapper starts a fresh process with the stock `proc`,
`return`, `catch`, `foreach`, `break` and `continue` commands. Only the recorded
releases and this direct invocation channel were measured; no Jim or host
embedding behaviour is established.

## Script

```tcl-visible
proc direct {code} {$code}
proc returned {code} {return -code $code}
foreach code {break continue} {
    set directStatus [catch {direct $code} directResult]
    set returnedStatus [catch {returned $code} returnedResult]
    puts [list direct $code $directStatus $directResult]
    puts [list returned $code $returnedStatus $returnedResult]
    set visited {}
    foreach item {1 2} {
        lappend visited before$item
        returned $code
        lappend visited after$item
    }
    puts [list caller-loop $code $visited]
}
```

## Results

### C Tcl 8.4.20

Invocation: `/workspace/toolchains/bin/tclsh8.4 /tmp/procedure-loop-completion.tcl`.

- Exit status: 0.
- Standard output:

```text-visible
direct break 1 {invoked "break" outside of a loop}
returned break 3 {}
caller-loop break before1
direct continue 1 {invoked "continue" outside of a loop}
returned continue 4 {}
caller-loop continue {before1 before2}
```

- Standard error: empty.

### C Tcl 8.5.19

Invocation: `/workspace/toolchains/bin/tclsh8.5 /tmp/procedure-loop-completion.tcl`.

- Exit status: 0.
- Standard output:

```text-visible
direct break 1 {invoked "break" outside of a loop}
returned break 3 {}
caller-loop break before1
direct continue 1 {invoked "continue" outside of a loop}
returned continue 4 {}
caller-loop continue {before1 before2}
```

- Standard error: empty.

### C Tcl 8.6.18

Invocation: `/workspace/toolchains/bin/tclsh8.6 /tmp/procedure-loop-completion.tcl`.

- Exit status: 0.
- Standard output:

```text-visible
direct break 1 {invoked "break" outside of a loop}
returned break 3 {}
caller-loop break before1
direct continue 1 {invoked "continue" outside of a loop}
returned continue 4 {}
caller-loop continue {before1 before2}
```

- Standard error: empty.

### C Tcl 9.0.4

Invocation: `/workspace/toolchains/bin/tclsh9.0 /tmp/procedure-loop-completion.tcl`.

- Exit status: 0.
- Standard output:

```text-visible
direct break 1 {invoked "break" outside of a loop}
returned break 3 {}
caller-loop break before1
direct continue 1 {invoked "continue" outside of a loop}
returned continue 4 {}
caller-loop continue {before1 before2}
```

- Standard error: empty.

### C Tcl 9.1.0

Invocation: `/workspace/toolchains/bin/tclsh9.1 /tmp/procedure-loop-completion.tcl`.

- Exit status: 0.
- Standard output:

```text-visible
direct break 1 {invoked "break" outside of a loop}
returned break 3 {}
caller-loop break before1
direct continue 1 {invoked "continue" outside of a loop}
returned continue 4 {}
caller-loop continue {before1 before2}
```

- Standard error: empty.
