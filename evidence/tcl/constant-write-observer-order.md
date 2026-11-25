# Callback order for operations on a constant variable

## Question

Which variable traces run when Tcl 9 rejects a write to a constant
scalar? How does `array unset` behave when the same scalar has an array
trace?

## Assumptions

Each invocation runs in a fresh Tcl process with the repository root as
its working directory. `LC_ALL` is `C` and `TCL_LIBRARY` selects the
matching pinned release's library. Other environment details were not
recorded. The probe records a command, its completion status and result,
and the trace operations it invoked.

## Script

```tcl-visible
proc watch {name index op} {lappend ::seen $op}
const scalar 3
const map {old 1}
foreach name {scalar map} {trace add variable $name {read write unset array} watch}
foreach script {
    {set scalar 4}
    {incr scalar nope}
    {append scalar text}
    {lappend scalar txt}
    {dict set map new a}
    {unset scalar}
    {array unset scalar}
} {
    set ::seen {}
    set status [catch $script message]
    puts [list $script $status $message $::seen]
}
```

## Results

### Tcl 9.0.4

- Launch argv: `tmp/tcl9.0.4/unix/tclsh` with no arguments.
- Input channel: standard input.
- Exit status: 0
- Standard output:

```text-visible
{set scalar 4} 1 {can't set "scalar": variable is a constant} {}
{incr scalar nope} 1 {can't incr "scalar": variable is a constant} {}
{append scalar text} 1 {can't set "scalar": variable is a constant} {}
{lappend scalar txt} 1 {can't set "scalar": variable is a constant} read
{dict set map new a} 1 {can't set "map": variable is a constant} read
{unset scalar} 1 {can't unset "scalar": variable is a constant} {}
{array unset scalar} 0 {} {}
```

- Standard error: empty.

### Tcl 9.1.0

- Launch argv: `tmp/tcl9.1.0/unix/tclsh` with no arguments.
- Input channel: standard input.
- Exit status: 0
- Standard output:

```text-visible
{set scalar 4} 1 {can't set "scalar": variable is a constant} {}
{incr scalar nope} 1 {can't incr "scalar": variable is a constant} {}
{append scalar text} 1 {can't set "scalar": variable is a constant} {}
{lappend scalar txt} 1 {can't set "scalar": variable is a constant} read
{dict set map new a} 1 {can't set "map": variable is a constant} read
{unset scalar} 1 {can't unset "scalar": variable is a constant} {}
{array unset scalar} 0 {} {}
```

- Standard error: empty.
