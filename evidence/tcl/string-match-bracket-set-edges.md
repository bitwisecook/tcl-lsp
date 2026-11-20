# `string match` bracket-set and backslash edge cases

## Question

How does `string match` treat a `-` before a set's closing `]` (`[a-]`), a set that is empty or begins with `]` (`[]`, `[]]`), a backslash inside a set (`[\]]`), an unterminated set (`[abc`, `x[`), a reversed range (`[z-a]`), an escaped backslash, and `*`/`?` against a newline? The F5 `matches` operator is this glob on BIG-IP, and the analyser translates it to a regular expression for F5 XC; the translation must agree with these results. This measures stock Tcl and Jim only.

## Assumptions

Each row used a fresh process on 2026-11-20 UTC with the exact script below as a script file argument and `LC_ALL` inherited. Other inherited environment details: Not recorded. The Tcl rows are the session's reference `tclsh` builds; the Jim row is the pinned oracle built by `scripts/dev/ensure-jim-oracle.sh`. Each output line is the Tcl list `pattern subject result`; newlines in the last two subjects print as line breaks.

## Script

```tcl-visible
foreach {p s} {{[a-]} - {[a-]} a {[a-]} b {[-a]} - {[]} ] {[]} {} {[]]} ] {[\]]} ] {[abc} a {[abc} abcd {a\\} {a\\} {[z-a]} m {x[} x {x[} {x[} {*} "a\nb" {?} "\n"} {
    puts [list $p $s [string match $p $s]]
}
```

## Results

### Tcl 8.4.20

- Launch argv: `tclsh8.4 glob.tcl`
- Input channel: script file.
- Command-line arguments: the script path.
- Exit status: 0
- Standard output:

```text-visible
{[a-]} - 0
{[a-]} a 1
{[a-]} b 0
{[-a]} - 1
{[]} \] 0
{[]} {} 0
{[]]} \] 0
{[\]]} \] 0
{[abc} a 1
{[abc} abcd 0
{a\\} {a\\} 0
{[z-a]} m 1
{x[} x 0
{x[} {x[} 0
* {a
b} 1
? {
} 1
```

- Standard error: empty

### Tcl 8.5.19

- Launch argv: `tclsh8.5 glob.tcl`
- Input channel: script file.
- Command-line arguments: the script path.
- Exit status: 0
- Standard output:

```text-visible
{[a-]} - 0
{[a-]} a 1
{[a-]} b 0
{[-a]} - 1
{[]} \] 0
{[]} {} 0
{[]]} \] 0
{[\]]} \] 0
{[abc} a 1
{[abc} abcd 0
{a\\} {a\\} 0
{[z-a]} m 1
{x[} x 0
{x[} {x[} 0
* {a
b} 1
? {
} 1
```

- Standard error: empty

### Tcl 8.6.18

- Launch argv: `tclsh8.6 glob.tcl`
- Input channel: script file.
- Command-line arguments: the script path.
- Exit status: 0
- Standard output:

```text-visible
{[a-]} - 0
{[a-]} a 1
{[a-]} b 0
{[-a]} - 1
{[]} \] 0
{[]} {} 0
{[]]} \] 0
{[\]]} \] 0
{[abc} a 1
{[abc} abcd 0
{a\\} {a\\} 0
{[z-a]} m 1
{x[} x 0
{x[} {x[} 0
* {a
b} 1
? {
} 1
```

- Standard error: empty

### Tcl 9.0.4

- Launch argv: `tclsh9.0 glob.tcl`
- Input channel: script file.
- Command-line arguments: the script path.
- Exit status: 0
- Standard output:

```text-visible
{[a-]} - 0
{[a-]} a 1
{[a-]} b 0
{[-a]} - 1
{[]} \] 0
{[]} {} 0
{[]]} \] 0
{[\]]} \] 0
{[abc} a 1
{[abc} abcd 0
{a\\} {a\\} 0
{[z-a]} m 1
{x[} x 0
{x[} {x[} 0
* {a
b} 1
? {
} 1
```

- Standard error: empty

### Tcl 9.1.0

- Launch argv: `tclsh9.1 glob.tcl`
- Input channel: script file.
- Command-line arguments: the script path.
- Exit status: 0
- Standard output:

```text-visible
{[a-]} - 0
{[a-]} a 1
{[a-]} b 0
{[-a]} - 1
{[]} \] 0
{[]} {} 0
{[]]} \] 0
{[\]]} \] 0
{[abc} a 1
{[abc} abcd 0
{a\\} {a\\} 0
{[z-a]} m 1
{x[} x 0
{x[} {x[} 0
* {a
b} 1
? {
} 1
```

- Standard error: empty

### Jim 0.84-15-g2f8e155

- Launch argv: `jimsh glob.tcl`
- Input channel: script file.
- Command-line arguments: the script path.
- Exit status: 0
- Standard output:

```text-visible
{[a-]} - 0
{[a-]} a 1
{[a-]} b 0
{[-a]} - 1
{[]} \] 0
{[]} {} 0
\[\]\] \] 0
{[\]]} \] 0
{[abc} a 1
{[abc} abcd 0
a\\\\ a\\\\ 0
{[z-a]} m 1
{x[} x 0
{x[} {x[} 0
* {a
b} 1
? {
} 1
```

- Standard error: empty
