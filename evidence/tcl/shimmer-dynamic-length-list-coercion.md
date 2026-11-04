# Tcl execution evidence

## Question

When a dynamic `string length` result is passed to `lindex`, what value is returned and, where the interpreter exposes its representation inspector, does the same object change from an integer to a list? This supports a narrowly scoped LSP conversion diagnostic; it does not establish language-server behaviour or a loop recurrence.

## Assumptions

Not recorded

The exact source-file CLI script below was run once per listed interpreter. The representation inspector is queried only where the interpreter exposes it; addresses are observations of this run, not stable identities across runs.

## Script

```tcl-visible
proc probe {s} {
    set n [string length $s]
    set before unavailable
    if {[llength [info commands tcl::unsupported::representation]]} {
        set before [tcl::unsupported::representation $n]
    }
    set result [lindex $n 0]
    set after unavailable
    if {[llength [info commands tcl::unsupported::representation]]} {
        set after [tcl::unsupported::representation $n]
    }
    return [list $result $before $after]
}
puts [list [info patchlevel] [probe abc]]
```

## Results

### tcl8.4

- Recorded version: 8.4.20
- Recorded input channel: Exact complete source-file command-line argument.
- Exit status: 0
- Standard output:

```text-visible
8.4.20 {3 unavailable unavailable}
```

- Standard error: empty.

### tcl8.5

- Recorded version: 8.5.19
- Recorded input channel: Exact complete source-file command-line argument.
- Exit status: 0
- Standard output:

```text-visible
8.5.19 {3 unavailable unavailable}
```

- Standard error: empty.

### tcl8.6

- Recorded version: 8.6.18
- Recorded input channel: Exact complete source-file command-line argument.
- Exit status: 0
- Standard output:

```text-visible
8.6.18 {3 {value is a int with a refcount of 2, object pointer at 0x556d1e24e820, internal representation 0x3:0x556d1e24e850, no string representation} {value is a list with a refcount of 2, object pointer at 0x556d1e24e820, internal representation 0x556d1e26e3e0:(nil), string representation "3"}}
```

- Standard error: empty.

### tcl9.0

- Recorded version: 9.0.4
- Recorded input channel: Exact complete source-file command-line argument.
- Exit status: 0
- Standard output:

```text-visible
9.0.4 {3 {value is a int with a refcount of 2, object pointer at 0x55a58ecb4a30, internal representation 0x3:0x55a58ecb61d0, no string representation} {value is a list with a refcount of 2, object pointer at 0x55a58ecb4a30, internal representation 0x55a58ece83b0:0x0, string representation "3"}}
```

- Standard error: empty.

### tcl9.1

- Recorded version: 9.1.0
- Recorded input channel: Exact complete source-file command-line argument.
- Exit status: 0
- Standard output:

```text-visible
9.1.0 {3 {value is a int with a refcount of 2, object pointer at 0x55e1a7b01900, internal representation 0x3:0x55e1a7b01990, no string representation} {value is a list with a refcount of 2, object pointer at 0x55e1a7b01900, internal representation 0x55e1a7b38120:0x0, string representation "3"}}
```

- Standard error: empty.

### Jim Tcl

- Recorded version: 0.84-10-g82fb4d0
- Recorded input channel: Exact complete source-file command-line argument.
- Exit status: 0
- Standard output:

```text-visible
0.84-10-g82fb4d0 {3 unavailable unavailable}
```

- Standard error: empty.
