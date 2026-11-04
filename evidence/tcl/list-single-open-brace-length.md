# Tcl execution evidence

## Question

Does a list consisting of one open brace cause `llength` to fail? This distinction matters when the LSP derives a conditional iteration count from original list values; the capture does not establish execution of a loop or language-server behaviour.

## Assumptions

Not recorded

The exact source-file CLI script below was run once per listed interpreter.

## Script

```tcl-visible
set value "\{"
puts [list [info patchlevel] [catch {llength $value} result] $result]
```

## Results

### tcl8.4

- Recorded version: 8.4.20
- Recorded input channel: Exact complete source-file command-line argument.
- Exit status: 0
- Standard output:

```text-visible
8.4.20 1 {unmatched open brace in list}
```

- Standard error: empty.

### tcl8.5

- Recorded version: 8.5.19
- Recorded input channel: Exact complete source-file command-line argument.
- Exit status: 0
- Standard output:

```text-visible
8.5.19 1 {unmatched open brace in list}
```

- Standard error: empty.

### tcl8.6

- Recorded version: 8.6.18
- Recorded input channel: Exact complete source-file command-line argument.
- Exit status: 0
- Standard output:

```text-visible
8.6.18 1 {unmatched open brace in list}
```

- Standard error: empty.

### tcl9.0

- Recorded version: 9.0.4
- Recorded input channel: Exact complete source-file command-line argument.
- Exit status: 0
- Standard output:

```text-visible
9.0.4 1 {unmatched open brace in list}
```

- Standard error: empty.

### tcl9.1

- Recorded version: 9.1.0
- Recorded input channel: Exact complete source-file command-line argument.
- Exit status: 0
- Standard output:

```text-visible
9.1.0 1 {unmatched open brace in list}
```

- Standard error: empty.

### Jim Tcl

- Recorded version: 0.84-10-g82fb4d0
- Recorded input channel: Exact complete source-file command-line argument.
- Exit status: 0
- Standard output:

```text-visible
0.84-10-g82fb4d0 0 1
```

- Standard error: empty.
