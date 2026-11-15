# Procedure extraction: motions the interpreter observes

## Question

When a command is moved from its frame into a new procedure that is then called, can the script observe the move through `info level`, through a write trace on a variable the moved command writes through `upvar`, or through `eval` of a computed script that creates a variable? The LSP's extract-procedure refactor must refuse a selection whose move changes what the script observes.

## Assumptions

Each row used a fresh process on 2026-11-15 UTC that received the exact script below on standard input with no command-line arguments. The interpreters are the pinned reference builds from `scripts/dev/tcl-reference-toolchains.sh`. Other environment details: Not recorded. The script sources no package.

## Script

```tcl-visible
puts [list VERSION [info patchlevel]]
proc lvl_extracted {} {return [info level]}
puts [list LEVEL before [info level] after [lvl_extracted]]
proc h args {lappend ::seen [info level]}
set v 0
trace add variable v write h
set v 1
proc tr_extracted {} {
    upvar 1 v v
    set v 2
}
tr_extracted
puts [list TRACE before [lindex $seen 0] after [lindex $seen 1]]
set script {set made 1}
proc ev_extracted {script} {
    eval $script
}
ev_extracted $script
set after [info exists made]
eval $script
puts [list EVAL before [info exists made] after $after]
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
LEVEL before 0 after 1
TRACE before 1 after 2
EVAL before 1 after 0
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
LEVEL before 0 after 1
TRACE before 1 after 2
EVAL before 1 after 0
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
LEVEL before 0 after 1
TRACE before 1 after 2
EVAL before 1 after 0
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
LEVEL before 0 after 1
TRACE before 1 after 2
EVAL before 1 after 0
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
LEVEL before 0 after 1
TRACE before 1 after 2
EVAL before 1 after 0
```

- Standard error: empty.

## Observation

On each recorded release the moved command reports a frame level one higher than the original (`LEVEL`), a write trace handler runs one level deeper when the traced write is made through the new procedure's `upvar` link (`TRACE`), and a variable created by `eval` of a computed script exists in the caller when evaluated in place and does not when evaluated inside the new procedure (`EVAL`). These rows do not establish behaviour for read traces, for command or execution traces, or for any release not listed.
