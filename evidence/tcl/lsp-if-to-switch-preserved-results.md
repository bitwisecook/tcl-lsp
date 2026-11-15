# If chain and switch -exact: results that agree

## Question

Does an `if`/`elseif` chain that compares one scalar with `eq` against string literals give the same result as `switch -exact --` on that scalar with the same bodies: for matching, non-matching, option-like and empty values, for `break` and `continue` in a body inside a loop, for no match without a default, and for an unset subject? The LSP's if-to-switch refactor rewrites exactly this shape.

## Assumptions

Each row used a fresh process on 2026-11-15 UTC that received the exact script below on standard input with no command-line arguments. The interpreters are the pinned reference builds from `scripts/dev/tcl-reference-toolchains.sh`. Other environment details: Not recorded. The script sources no package.

## Script

```tcl-visible
puts [list VERSION [info patchlevel]]
proc as_if {x} {
    if {$x eq "a"} {
        return alpha
    } elseif {$x eq "b c"} {
        return beta
    } else {
        return other
    }
}
proc as_switch {x} {
    switch -exact -- $x {
        a {
            return alpha
        }
        {b c} {
            return beta
        }
        default {
            return other
        }
    }
}
foreach value {a {b c} A -x {} 1.0} {
    puts [list VALUE $value if [as_if $value] switch [as_switch $value]]
}
proc loop_if {} {
    set seen {}
    foreach x {a b c} {
        if {$x eq "a"} {continue} elseif {$x eq "b"} {break}
        lappend seen $x
    }
    return [list $x $seen]
}
proc loop_switch {} {
    set seen {}
    foreach x {a b c} {
        switch -exact -- $x {
            a {continue}
            b {break}
        }
        lappend seen $x
    }
    return [list $x $seen]
}
puts [list LOOP if [loop_if] switch [loop_switch]]
proc none_if {x} {if {$x eq "a"} {set r 1} elseif {$x eq "b"} {set r 2}}
proc none_switch {x} {switch -exact -- $x {a {set r 1} b {set r 2}}}
puts [list NOMATCH if [list [none_if z]] switch [list [none_switch z]] HIT if [none_if b] switch [none_switch b]]
puts [list UNSET if [catch {if {$nope eq "a"} {set r 1} elseif {$nope eq "b"} {set r 2}} m] $m switch [catch {switch -exact -- $nope {a {set r 1} b {set r 2}}} m] $m]
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
VALUE a if alpha switch alpha
VALUE {b c} if beta switch beta
VALUE A if other switch other
VALUE -x if other switch other
VALUE {} if other switch other
VALUE 1.0 if other switch other
LOOP if {b {}} switch {b {}}
NOMATCH if {{}} switch {{}} HIT if 2 switch 2
UNSET if 1 {can't read "nope": no such variable} switch 1 {can't read "nope": no such variable}
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
VALUE a if alpha switch alpha
VALUE {b c} if beta switch beta
VALUE A if other switch other
VALUE -x if other switch other
VALUE {} if other switch other
VALUE 1.0 if other switch other
LOOP if {b {}} switch {b {}}
NOMATCH if {{}} switch {{}} HIT if 2 switch 2
UNSET if 1 {can't read "nope": no such variable} switch 1 {can't read "nope": no such variable}
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
VALUE a if alpha switch alpha
VALUE {b c} if beta switch beta
VALUE A if other switch other
VALUE -x if other switch other
VALUE {} if other switch other
VALUE 1.0 if other switch other
LOOP if {b {}} switch {b {}}
NOMATCH if {{}} switch {{}} HIT if 2 switch 2
UNSET if 1 {can't read "nope": no such variable} switch 1 {can't read "nope": no such variable}
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
VALUE a if alpha switch alpha
VALUE {b c} if beta switch beta
VALUE A if other switch other
VALUE -x if other switch other
VALUE {} if other switch other
VALUE 1.0 if other switch other
LOOP if {b {}} switch {b {}}
NOMATCH if {{}} switch {{}} HIT if 2 switch 2
UNSET if 1 {can't read "nope": no such variable} switch 1 {can't read "nope": no such variable}
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
VALUE a if alpha switch alpha
VALUE {b c} if beta switch beta
VALUE A if other switch other
VALUE -x if other switch other
VALUE {} if other switch other
VALUE 1.0 if other switch other
LOOP if {b {}} switch {b {}}
NOMATCH if {{}} switch {{}} HIT if 2 switch 2
UNSET if 1 {can't read "nope": no such variable} switch 1 {can't read "nope": no such variable}
```

- Standard error: empty.

## Observation

On each recorded release the two forms return the same value for every listed subject (`VALUE`), leave a loop in the same state after `continue` and `break` in a body (`LOOP`), return the empty string when nothing matches and the body's value when something does (`NOMATCH`, `HIT`), and raise the same error for an unset subject (`UNSET`). The script sets no trace; see lsp-if-to-switch-read-count.md for the difference a read trace observes.
