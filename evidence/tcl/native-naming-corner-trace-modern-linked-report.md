# Tcl naming: trace modern linked report

## Question

What root and element do modern read traces report through this local alias? This constrains name resolution and binding behavior in the LSP, compiler and VM for the tested inputs.

## Assumptions

Each recorded case was executed as a file in a separate shell process. The script catches the guest operation and prints its completion code and result bytes as hex; that Tcl completion is distinct from the shell process exit status. Results apply to the listed environments and inputs; BIG-IP was not part of this capture. Other assumptions were not separately recorded.

## Script

```tcl-visible
set code [catch {set a(k) VALUE; set log {}; proc observe {n e op} {lappend ::log [list $n $e $op]}; proc p {} {upvar #0 a(k) alias; trace add variable alias read observe; set alias}; list [p] $log} result]
binary scan $result H* hex
puts [list $code $hex]
```

## Results

### 8.4.20

- Recorded version: 8.4.20.
- Recorded input channel: file argument to `/tmp/2286-oracles/bin/tclsh8.4`.
- Exit status: 0.
- Standard output:

```text-visible
0 56414c5545207b7b616c696173207b7d20726561647d7d
```

- Standard error: empty.

The script recorded Tcl completion code 0. Its hex-encoded guest result, decoded for reading:

```text-visible
VALUE {{alias {} read}}⟦EOF⟧
```

### 8.5.19

- Recorded version: 8.5.19.
- Recorded input channel: file argument to `/tmp/2286-oracles/bin/tclsh8.5`.
- Exit status: 0.
- Standard output:

```text-visible
0 56414c5545207b7b616c696173207b7d20726561647d7d
```

- Standard error: empty.

The script recorded Tcl completion code 0. Its hex-encoded guest result, decoded for reading:

```text-visible
VALUE {{alias {} read}}⟦EOF⟧
```

### 8.6.18

- Recorded version: 8.6.18.
- Recorded input channel: file argument to `/tmp/2286-oracles/bin/tclsh8.6`.
- Exit status: 0.
- Standard output:

```text-visible
0 56414c5545207b7b616c696173207b7d20726561647d7d
```

- Standard error: empty.

The script recorded Tcl completion code 0. Its hex-encoded guest result, decoded for reading:

```text-visible
VALUE {{alias {} read}}⟦EOF⟧
```

### 9.0.4

- Recorded version: 9.0.4.
- Recorded input channel: file argument to `/tmp/2286-oracles/bin/tclsh9.0`.
- Exit status: 0.
- Standard output:

```text-visible
0 56414c5545207b7b616c696173206b20726561647d7d
```

- Standard error: empty.

The script recorded Tcl completion code 0. Its hex-encoded guest result, decoded for reading:

```text-visible
VALUE {{alias k read}}⟦EOF⟧
```

### 9.1.0

- Recorded version: 9.1.0.
- Recorded input channel: file argument to `/tmp/2286-oracles/bin/tclsh9.1`.
- Exit status: 0.
- Standard output:

```text-visible
0 56414c5545207b7b616c696173206b20726561647d7d
```

- Standard error: empty.

The script recorded Tcl completion code 0. Its hex-encoded guest result, decoded for reading:

```text-visible
VALUE {{alias k read}}⟦EOF⟧
```

### Jim

- Recorded version: 0.84-9-g5bac7c9.
- Recorded input channel: file argument to `/tmp/2286-oracles/jimtcl/jimsh`.
- Exit status: 0.
- Standard output:

```text-visible
1 696e76616c696420636f6d6d616e64206e616d652022747261636522
```

- Standard error: empty.

The script recorded Tcl completion code 1. Its hex-encoded guest result, decoded for reading:

```text-visible
invalid command name "trace"⟦EOF⟧
```
