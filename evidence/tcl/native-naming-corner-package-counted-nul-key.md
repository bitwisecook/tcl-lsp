# Tcl naming: package counted nul key

## Question

Does package present distinguish this generated NUL-bearing package name from Pkg? This constrains name resolution and binding behavior in the LSP, compiler and VM for the tested inputs.

## Assumptions

Each recorded case was executed as a file in a separate shell process. The script catches the guest operation and prints its completion code and result bytes as hex; that Tcl completion is distinct from the shell process exit status. Results apply to the listed environments and inputs; BIG-IP was not part of this capture. Other assumptions were not separately recorded.

## Script

```tcl-visible
set code [catch {set name "Pkg[format %c 0]Tail"; package provide $name 1.0; list [package present $name] [catch {package present Pkg}]} result]
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
0 312e302031
```

- Standard error: empty.

The script recorded Tcl completion code 0. Its hex-encoded guest result, decoded for reading:

```text-visible
1.0 1⟦EOF⟧
```

### 8.5.19

- Recorded version: 8.5.19.
- Recorded input channel: file argument to `/tmp/2286-oracles/bin/tclsh8.5`.
- Exit status: 0.
- Standard output:

```text-visible
0 312e302031
```

- Standard error: empty.

The script recorded Tcl completion code 0. Its hex-encoded guest result, decoded for reading:

```text-visible
1.0 1⟦EOF⟧
```

### 8.6.18

- Recorded version: 8.6.18.
- Recorded input channel: file argument to `/tmp/2286-oracles/bin/tclsh8.6`.
- Exit status: 0.
- Standard output:

```text-visible
0 312e302031
```

- Standard error: empty.

The script recorded Tcl completion code 0. Its hex-encoded guest result, decoded for reading:

```text-visible
1.0 1⟦EOF⟧
```

### 9.0.4

- Recorded version: 9.0.4.
- Recorded input channel: file argument to `/tmp/2286-oracles/bin/tclsh9.0`.
- Exit status: 0.
- Standard output:

```text-visible
0 312e302031
```

- Standard error: empty.

The script recorded Tcl completion code 0. Its hex-encoded guest result, decoded for reading:

```text-visible
1.0 1⟦EOF⟧
```

### 9.1.0

- Recorded version: 9.1.0.
- Recorded input channel: file argument to `/tmp/2286-oracles/bin/tclsh9.1`.
- Exit status: 0.
- Standard output:

```text-visible
0 312e302031
```

- Standard error: empty.

The script recorded Tcl completion code 0. Its hex-encoded guest result, decoded for reading:

```text-visible
1.0 1⟦EOF⟧
```

### Jim

- Recorded version: 0.84-9-g5bac7c9.
- Recorded input channel: file argument to `/tmp/2286-oracles/jimtcl/jimsh`.
- Exit status: 0.
- Standard output:

```text-visible
1 7061636b6167652c20756e6b6e6f776e20636f6d6d616e64202270726573656e74223a2073686f756c6420626520666f726765742c206e616d65732c2070726f766964652c2072657175697265
```

- Standard error: empty.

The script recorded Tcl completion code 1. Its hex-encoded guest result, decoded for reading:

```text-visible
package, unknown command "present": should be forget, names, provide, require⟦EOF⟧
```
