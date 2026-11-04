# Tcl variable operand: trace qualified index trace report

## Question

What variable and index names are reported to a trace on a qualified array element accessed by lappend? This constrains array-name resolution and trace dispatch in the LSP, compiler and VM. This script uses the legacy trace interface.

## Assumptions

The recorded programs use a computed command name and the name values constructed by the script. Binary-NUL and decomposed names are produced at runtime, not literal non-ASCII source bytes. The script catches some Tcl errors and encodes result bytes as hex; those caught errors are distinct from the shell's exit status. The capture records launcher arguments but does not separately identify the script input channel. Other assumptions were not separately recorded.

## Script

```tcl-visible
proc hex {s} {binary scan $s H* out; return $out}
set c [catch {namespace eval N {}; set log {}; proc watch {n k op} {global log; lappend log [list $n $k $op]}; set n {::N::arr(k::part)}; trace variable $n rw watch; set h lappend; $h $n A B; list $log [set $n]} r]
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
0 7b7b3a3a4e3a3a617272206b3a3a7061727420727d207b3a3a4e3a3a617272206b3a3a7061727420777d7d207b4120427d
```

- Standard error: empty.

The script's hex-encoded inner command result, decoded for reading:

```text-visible
{{::N::arr k::part r} {::N::arr k::part w}} {A B}⟦EOF⟧
```

### tcl8.5

- Recorded version: 8.5.19.
- Recorded input channel: Not separately recorded; launcher `/tmp/2286-oracles/bin/tclsh8.5`.
- Exit status: 0.
- Standard output:

```text-visible
0 7b7b3a3a4e3a3a617272206b3a3a7061727420727d207b3a3a4e3a3a617272206b3a3a7061727420777d7d207b4120427d
```

- Standard error: empty.

The script's hex-encoded inner command result, decoded for reading:

```text-visible
{{::N::arr k::part r} {::N::arr k::part w}} {A B}⟦EOF⟧
```

### tcl8.6

- Recorded version: 8.6.18.
- Recorded input channel: Not separately recorded; launcher `/tmp/2286-oracles/bin/tclsh8.6`.
- Exit status: 0.
- Standard output:

```text-visible
0 7b7b3a3a4e3a3a617272206b3a3a7061727420727d207b3a3a4e3a3a617272206b3a3a7061727420777d7d207b4120427d
```

- Standard error: empty.

The script's hex-encoded inner command result, decoded for reading:

```text-visible
{{::N::arr k::part r} {::N::arr k::part w}} {A B}⟦EOF⟧
```

### tcl9.0

- Recorded version: 9.0.4.
- Recorded input channel: Not separately recorded; launcher `/tmp/2286-oracles/bin/tclsh9.0`.
- Exit status: 0.
- Standard output:

```text-visible
1 626164206f7074696f6e20227661726961626c65223a206d757374206265206164642c20696e666f2c206f722072656d6f7665
```

- Standard error: empty.

The script's hex-encoded inner command result, decoded for reading:

```text-visible
bad option "variable": must be add, info, or remove⟦EOF⟧
```

### tcl9.1

- Recorded version: 9.1.0.
- Recorded input channel: Not separately recorded; launcher `/tmp/2286-oracles/bin/tclsh9.1`.
- Exit status: 0.
- Standard output:

```text-visible
1 626164206f7074696f6e20227661726961626c65223a206d757374206265206164642c20696e666f2c206f722072656d6f7665
```

- Standard error: empty.

The script's hex-encoded inner command result, decoded for reading:

```text-visible
bad option "variable": must be add, info, or remove⟦EOF⟧
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
