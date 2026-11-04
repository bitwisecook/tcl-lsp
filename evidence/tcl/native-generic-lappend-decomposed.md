# Tcl variable operand: lappend decomposed

## Question

How does lappend resolve and update a dynamically constructed decomposed Unicode name when the command is invoked through a computed command name? This constrains name resolution and output-variable semantics in the LSP, compiler and VM.

## Assumptions

The recorded programs use a computed command name and the name values constructed by the script. Binary-NUL and decomposed names are produced at runtime, not literal non-ASCII source bytes. The script catches some Tcl errors and encodes result bytes as hex; those caught errors are distinct from the shell's exit status. The capture records launcher arguments but does not separately identify the script input channel. Other assumptions were not separately recorded.

## Script

```tcl-visible
proc hex {s} {binary scan $s H* out; return $out}
namespace eval N {}
set n "e\u0301"
set c [catch {set $n A; set h lappend; $h $n B C} r]
set exists [info exists $n]
set rc [catch {set $n} got]
set summary [list $c [hex $r] $exists $rc [hex $got]]
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
0 4120422043 1 0 4120422043
```

- Standard error: empty.

The script's hex-encoded inner command result, decoded for reading:

```text-visible
A B C⟦EOF⟧
```

The script's hex-encoded final variable-read result, decoded for reading:

```text-visible
A B C⟦EOF⟧
```

### tcl8.5

- Recorded version: 8.5.19.
- Recorded input channel: Not separately recorded; launcher `/tmp/2286-oracles/bin/tclsh8.5`.
- Exit status: 0.
- Standard output:

```text-visible
0 4120422043 1 0 4120422043
```

- Standard error: empty.

The script's hex-encoded inner command result, decoded for reading:

```text-visible
A B C⟦EOF⟧
```

The script's hex-encoded final variable-read result, decoded for reading:

```text-visible
A B C⟦EOF⟧
```

### tcl8.6

- Recorded version: 8.6.18.
- Recorded input channel: Not separately recorded; launcher `/tmp/2286-oracles/bin/tclsh8.6`.
- Exit status: 0.
- Standard output:

```text-visible
0 4120422043 1 0 4120422043
```

- Standard error: empty.

The script's hex-encoded inner command result, decoded for reading:

```text-visible
A B C⟦EOF⟧
```

The script's hex-encoded final variable-read result, decoded for reading:

```text-visible
A B C⟦EOF⟧
```

### tcl9.0

- Recorded version: 9.0.4.
- Recorded input channel: Not separately recorded; launcher `/tmp/2286-oracles/bin/tclsh9.0`.
- Exit status: 0.
- Standard output:

```text-visible
0 4120422043 1 0 4120422043
```

- Standard error: empty.

The script's hex-encoded inner command result, decoded for reading:

```text-visible
A B C⟦EOF⟧
```

The script's hex-encoded final variable-read result, decoded for reading:

```text-visible
A B C⟦EOF⟧
```

### tcl9.1

- Recorded version: 9.1.0.
- Recorded input channel: Not separately recorded; launcher `/tmp/2286-oracles/bin/tclsh9.1`.
- Exit status: 0.
- Standard output:

```text-visible
0 4120422043 1 0 4120422043
```

- Standard error: empty.

The script's hex-encoded inner command result, decoded for reading:

```text-visible
A B C⟦EOF⟧
```

The script's hex-encoded final variable-read result, decoded for reading:

```text-visible
A B C⟦EOF⟧
```

### jim

- Recorded version: 0.84-9-g5bac7c9.
- Recorded input channel: Not separately recorded; launcher `/tmp/2286-oracles/jimtcl/jimsh`.
- Exit status: 0.
- Standard output:

```text-visible
0 4120422043 1 0 4120422043
```

- Standard error: empty.

The script's hex-encoded inner command result, decoded for reading:

```text-visible
A B C⟦EOF⟧
```

The script's hex-encoded final variable-read result, decoded for reading:

```text-visible
A B C⟦EOF⟧
```
