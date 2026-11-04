# Tcl variable operand: info default decomposed

## Question

How does info default output assignment resolve and update a dynamically constructed decomposed Unicode name when the command is invoked through a computed command name? This constrains name resolution and output-variable semantics in the LSP, compiler and VM.

## Assumptions

The recorded programs use a computed command name and the name values constructed by the script. Binary-NUL and decomposed names are produced at runtime, not literal non-ASCII source bytes. The script catches some Tcl errors and encodes result bytes as hex; those caught errors are distinct from the shell's exit status. The capture records launcher arguments but does not separately identify the script input channel. Other assumptions were not separately recorded.

## Script

```tcl-visible
proc hex {s} {binary scan $s H* out; return $out}
namespace eval N {}
set n "e\u0301"
set c [catch {proc sample {{x DEFAULT}} {}; set h info; $h default sample x $n} r]
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
0 31 1 0 44454641554c54
```

- Standard error: empty.

The script's hex-encoded inner command result, decoded for reading:

```text-visible
1⟦EOF⟧
```

The script's hex-encoded final variable-read result, decoded for reading:

```text-visible
DEFAULT⟦EOF⟧
```

### tcl8.5

- Recorded version: 8.5.19.
- Recorded input channel: Not separately recorded; launcher `/tmp/2286-oracles/bin/tclsh8.5`.
- Exit status: 0.
- Standard output:

```text-visible
0 31 1 0 44454641554c54
```

- Standard error: empty.

The script's hex-encoded inner command result, decoded for reading:

```text-visible
1⟦EOF⟧
```

The script's hex-encoded final variable-read result, decoded for reading:

```text-visible
DEFAULT⟦EOF⟧
```

### tcl8.6

- Recorded version: 8.6.18.
- Recorded input channel: Not separately recorded; launcher `/tmp/2286-oracles/bin/tclsh8.6`.
- Exit status: 0.
- Standard output:

```text-visible
0 31 1 0 44454641554c54
```

- Standard error: empty.

The script's hex-encoded inner command result, decoded for reading:

```text-visible
1⟦EOF⟧
```

The script's hex-encoded final variable-read result, decoded for reading:

```text-visible
DEFAULT⟦EOF⟧
```

### tcl9.0

- Recorded version: 9.0.4.
- Recorded input channel: Not separately recorded; launcher `/tmp/2286-oracles/bin/tclsh9.0`.
- Exit status: 0.
- Standard output:

```text-visible
0 31 1 0 44454641554c54
```

- Standard error: empty.

The script's hex-encoded inner command result, decoded for reading:

```text-visible
1⟦EOF⟧
```

The script's hex-encoded final variable-read result, decoded for reading:

```text-visible
DEFAULT⟦EOF⟧
```

### tcl9.1

- Recorded version: 9.1.0.
- Recorded input channel: Not separately recorded; launcher `/tmp/2286-oracles/bin/tclsh9.1`.
- Exit status: 0.
- Standard output:

```text-visible
0 31 1 0 44454641554c54
```

- Standard error: empty.

The script's hex-encoded inner command result, decoded for reading:

```text-visible
1⟦EOF⟧
```

The script's hex-encoded final variable-read result, decoded for reading:

```text-visible
DEFAULT⟦EOF⟧
```

### jim

- Recorded version: 0.84-9-g5bac7c9.
- Recorded input channel: Not separately recorded; launcher `/tmp/2286-oracles/jimtcl/jimsh`.
- Exit status: 0.
- Standard output:

```text-visible
1 696e666f2c20756e6b6e6f776e20636f6d6d616e64202264656661756c74223a2073686f756c6420626520616c6961732c20616c69617365732c20617267732c20626f64792c206368616e6e656c732c20636f6d6d616e64732c20636f6d706c6574652c206578697374732c206672616d652c20676c6f62616c732c2068656c702c20686f73746e616d652c206c6576656c2c206c6f63616c732c206e616d656f6665786563757461626c652c2070617463686c6576656c2c2070726f63732c207265666572656e6365732c2072657475726e636f6465732c207363726970742c20736f757263652c20737461636b74726163652c20737461746963732c207461696e7465642c2075736167652c20766172732c2076657273696f6e 0 1 63616e27742072656164202265cc81223a206e6f2073756368207661726961626c65
```

- Standard error: empty.

The script's hex-encoded inner command result, decoded for reading:

```text-visible
info, unknown command "default": should be alias, aliases, args, body, channels, commands, complete, exists, frame, globals, help, hostname, level, locals, nameofexecutable, patchlevel, procs, references, returncodes, script, source, stacktrace, statics, tainted, usage, vars, version⟦EOF⟧
```

The script's hex-encoded final variable-read result, decoded for reading:

```text-visible
can't read "é": no such variable⟦EOF⟧
```
