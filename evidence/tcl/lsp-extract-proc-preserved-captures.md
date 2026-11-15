# Procedure extraction: captures that preserve behaviour

## Question

When a command is moved into a new procedure, does passing a variable it only reads as a value parameter, and linking a variable it writes with `upvar 1`, leave the values the script observes unchanged? The LSP's extract-procedure refactor produces exactly these two capture forms, including for a write inside a loop body.

## Assumptions

Each row used a fresh process on 2026-11-15 UTC that received the exact script below on standard input with no command-line arguments. The interpreters are the pinned reference builds from `scripts/dev/tcl-reference-toolchains.sh`. Other environment details: Not recorded. The script sources no package.

## Script

```tcl-visible
puts [list VERSION [info patchlevel]]
set x 5
set before [list [expr {$x + 1}]]
proc read_extracted {x} {
    return [expr {$x + 1}]
}
puts [list READ before $before after [list [read_extracted $x]]]
set y 0
set y 1
set before $y
set y 0
proc write_extracted {} {
    upvar 1 y y
    set y 1
}
write_extracted
puts [list WRITE before $before after $y]
set total 0
foreach n {1 2 3} {incr total $n}
set before $total
set total 0
proc loop_extracted {} {
    upvar 1 total total
    foreach n {1 2 3} {incr total $n}
}
loop_extracted
puts [list LOOP before $before after $total]
```

## Results

### Tcl 8.4.20

- Launch argv: `/root/.local/bin/tclsh8.4`
- Input channel: standard input.
- Command-line arguments: none.
- Exit status: 0
- Standard output:

```text-visible
VERSION 8.4.20
READ before 6 after 6
WRITE before 1 after 1
LOOP before 6 after 6
```

- Standard error: empty.

### Tcl 8.5.19

- Launch argv: `/root/.local/bin/tclsh8.5`
- Input channel: standard input.
- Command-line arguments: none.
- Exit status: 0
- Standard output:

```text-visible
VERSION 8.5.19
READ before 6 after 6
WRITE before 1 after 1
LOOP before 6 after 6
```

- Standard error: empty.

### Tcl 8.6.18

- Launch argv: `/root/.local/bin/tclsh8.6`
- Input channel: standard input.
- Command-line arguments: none.
- Exit status: 0
- Standard output:

```text-visible
VERSION 8.6.18
READ before 6 after 6
WRITE before 1 after 1
LOOP before 6 after 6
```

- Standard error: empty.

### Tcl 9.0.4

- Launch argv: `/root/.local/bin/tclsh9.0`
- Input channel: standard input.
- Command-line arguments: none.
- Exit status: 0
- Standard output:

```text-visible
VERSION 9.0.4
READ before 6 after 6
WRITE before 1 after 1
LOOP before 6 after 6
```

- Standard error: empty.

### Tcl 9.1.0

- Launch argv: `/root/.local/bin/tclsh9.1`
- Input channel: standard input.
- Command-line arguments: none.
- Exit status: 0
- Standard output:

```text-visible
VERSION 9.1.0
READ before 6 after 6
WRITE before 1 after 1
LOOP before 6 after 6
```

- Standard error: empty.

## Observation

On each recorded release the value computed from a read-only capture passed as a parameter (`READ`), a write carried by `upvar 1` (`WRITE`) and a loop that accumulates into an `upvar`-linked variable (`LOOP`) equal the values computed in place. The script sets no trace and uses no frame introspection; these rows do not establish that the forms preserve behaviour where such an observer exists.
