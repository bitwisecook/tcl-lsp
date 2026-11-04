# Tcl variable operand: trace modern sequential output link rebind

## Question

Can a trace on the first scan output rebind the second output variable while scan is assigning results? This constrains output-variable lookup and links in the compiler and VM. This script uses trace add/remove.

## Assumptions

The recorded programs use a computed command name and the name values constructed by the script. Binary-NUL and decomposed names are produced at runtime, not literal non-ASCII source bytes. The script catches some Tcl errors and encodes result bytes as hex; those caught errors are distinct from the shell's exit status. The capture records launcher arguments but does not separately identify the script input channel. Other assumptions were not separately recorded.

## Script

```tcl-visible
proc hex {s} {binary scan $s H* out; return $out}
set c [catch {set replacement BEFORE; set first INIT; set second ORIGINAL; proc watch {n k op} {uplevel 1 {unset second; upvar #0 replacement second}}; trace add variable first write watch; set n first; set m second; set h scan; set count [$h {7 9} {%d %d} $n $m]; list $count $first $second $replacement} r]
set summary [list $c [hex $r]]
puts $summary
set summary
```

## Results

### tcl8.4

- Recorded version: 8.4.20.
- Recorded input channel: Not separately recorded; launcher `/tmp/2286-oracles/bin/tclsh8.4`.
- Exit status: 0.
- Standard output:

```text-visible
0 32203720392039
```

- Standard error: empty.

The script's hex-encoded inner command result, decoded for reading:

```text-visible
2 7 9 9⟦EOF⟧
```

### tcl8.5

- Recorded version: 8.5.19.
- Recorded input channel: Not separately recorded; launcher `/tmp/2286-oracles/bin/tclsh8.5`.
- Exit status: 0.
- Standard output:

```text-visible
0 32203720392039
```

- Standard error: empty.

The script's hex-encoded inner command result, decoded for reading:

```text-visible
2 7 9 9⟦EOF⟧
```

### tcl8.6

- Recorded version: 8.6.18.
- Recorded input channel: Not separately recorded; launcher `/tmp/2286-oracles/bin/tclsh8.6`.
- Exit status: 0.
- Standard output:

```text-visible
0 32203720392039
```

- Standard error: empty.

The script's hex-encoded inner command result, decoded for reading:

```text-visible
2 7 9 9⟦EOF⟧
```

### tcl9.0

- Recorded version: 9.0.4.
- Recorded input channel: Not separately recorded; launcher `/tmp/2286-oracles/bin/tclsh9.0`.
- Exit status: 0.
- Standard output:

```text-visible
0 32203720392039
```

- Standard error: empty.

The script's hex-encoded inner command result, decoded for reading:

```text-visible
2 7 9 9⟦EOF⟧
```

### tcl9.1

- Recorded version: 9.1.0.
- Recorded input channel: Not separately recorded; launcher `/tmp/2286-oracles/bin/tclsh9.1`.
- Exit status: 0.
- Standard output:

```text-visible
0 32203720392039
```

- Standard error: empty.

The script's hex-encoded inner command result, decoded for reading:

```text-visible
2 7 9 9⟦EOF⟧
```

### jim

- Recorded version: 0.84-9-g5bac7c9.
- Recorded input channel: Not separately recorded; launcher `/tmp/2286-oracles/jimtcl/jimsh`.
- Exit status: 0.
- Standard output:

```text-visible
1 696e76616c696420636f6d6d616e64206e616d652022747261636522
```

- Standard error: empty.

The script's hex-encoded inner command result, decoded for reading:

```text-visible
invalid command name "trace"⟦EOF⟧
```
