# Tcl naming: oo counted nul object

## Question

Can this dynamically generated NUL-bearing object command select its method? This constrains name resolution and binding behavior in the LSP, compiler and VM for the tested inputs.

## Assumptions

Each recorded case was executed as a file in a separate shell process. The script catches the guest operation and prints its completion code and result bytes as hex; that Tcl completion is distinct from the shell process exit status. Results apply to the listed environments and inputs; BIG-IP was not part of this capture. Other assumptions were not separately recorded.

## Script

```tcl-visible
set code [catch {oo::class create C {method get {} {return VALUE}}; set name "object[format %c 0]tail"; C create $name; eval [list $name get]} result]
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
1 696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
```

- Standard error: empty.

The script recorded Tcl completion code 1. Its hex-encoded guest result, decoded for reading:

```text-visible
invalid command name "oo::class"⟦EOF⟧
```

### 8.5.19

- Recorded version: 8.5.19.
- Recorded input channel: file argument to `/tmp/2286-oracles/bin/tclsh8.5`.
- Exit status: 0.
- Standard output:

```text-visible
1 696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
```

- Standard error: empty.

The script recorded Tcl completion code 1. Its hex-encoded guest result, decoded for reading:

```text-visible
invalid command name "oo::class"⟦EOF⟧
```

### 8.6.18

- Recorded version: 8.6.18.
- Recorded input channel: file argument to `/tmp/2286-oracles/bin/tclsh8.6`.
- Exit status: 0.
- Standard output:

```text-visible
0 56414c5545
```

- Standard error: empty.

The script recorded Tcl completion code 0. Its hex-encoded guest result, decoded for reading:

```text-visible
VALUE⟦EOF⟧
```

### 9.0.4

- Recorded version: 9.0.4.
- Recorded input channel: file argument to `/tmp/2286-oracles/bin/tclsh9.0`.
- Exit status: 0.
- Standard output:

```text-visible
0 56414c5545
```

- Standard error: empty.

The script recorded Tcl completion code 0. Its hex-encoded guest result, decoded for reading:

```text-visible
VALUE⟦EOF⟧
```

### 9.1.0

- Recorded version: 9.1.0.
- Recorded input channel: file argument to `/tmp/2286-oracles/bin/tclsh9.1`.
- Exit status: 0.
- Standard output:

```text-visible
0 56414c5545
```

- Standard error: empty.

The script recorded Tcl completion code 0. Its hex-encoded guest result, decoded for reading:

```text-visible
VALUE⟦EOF⟧
```

### Jim

- Recorded version: 0.84-9-g5bac7c9.
- Recorded input channel: file argument to `/tmp/2286-oracles/jimtcl/jimsh`.
- Exit status: 0.
- Standard output:

```text-visible
1 696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
```

- Standard error: empty.

The script recorded Tcl completion code 1. Its hex-encoded guest result, decoded for reading:

```text-visible
invalid command name "oo::class"⟦EOF⟧
```
