# If chain and switch -exact: how often the subject is read

## Question

How many times is the subject variable read by an `if`/`elseif` chain whose second condition matches, and by the equivalent `switch -exact --`? A read trace on the subject observes the count, so the LSP's if-to-switch refactor must refuse where such a trace can exist.

## Assumptions

Each row used a fresh process on 2026-11-15 UTC that received the exact script below on standard input with no command-line arguments. The interpreters are the pinned reference builds from `scripts/dev/tcl-reference-toolchains.sh`. Other environment details: Not recorded. The script sources no package.

## Script

```tcl-visible
puts [list VERSION [info patchlevel]]
proc count args {incr ::reads}
set x b
trace add variable x read count
set reads 0
if {$x eq "a"} {set r 1} elseif {$x eq "b"} {set r 2}
set if_reads $reads
set reads 0
switch -exact -- $x {a {set r 1} b {set r 2}}
puts [list READS if $if_reads switch $reads]
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
READS if 2 switch 1
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
READS if 2 switch 1
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
READS if 2 switch 1
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
READS if 2 switch 1
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
READS if 2 switch 1
```

- Standard error: empty.

## Observation

On each recorded release the chain reads the subject twice (once per evaluated condition) and the switch reads it once. These rows do not establish the count for other chain lengths or match positions.
