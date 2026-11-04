# Procedure names beginning with a colon

## Question

When a procedure name begins with a colon, does Tcl accept its declaration in a non-global namespace and in the global namespace, and how does that compare with an ordinary local name? This distinction affects namespace-aware name resolution and declaration diagnostics in the LSP and compiler.

## Assumptions

The script catches three declarations and prints their completion codes and result text. It does not invoke the resulting procedure bodies. The capture identifies interpreter release series but did not record the full launched patchlevels or whether the source was supplied on standard input or as a file. BIG-IP was not tested. Other assumptions were not separately recorded.

## Script

```tcl-visible
namespace eval ::ns {}
set code [catch {namespace eval ::ns {proc :p {} {return X}}} value]
set hex {};foreach ch [split $value {}] {scan $ch %c number;append hex [format %02x $number]}
puts "local-colon\t$code\t$hex"
set code [catch {namespace eval :: {proc :p {} {return X}}} value]
set hex {};foreach ch [split $value {}] {scan $ch %c number;append hex [format %02x $number]}
puts "root-colon\t$code\t$hex"
set code [catch {namespace eval ::ns {proc p {} {return X}}} value]
set hex {};foreach ch [split $value {}] {scan $ch %c number;append hex [format %02x $number]}
puts "local-plain\t$code\t$hex"
```

## Results

### 8.4

- Recorded version: 8.4; full patchlevel Not recorded.
- Recorded input channel: Not recorded.
- Exit status: 0
- Standard output:

```text-visible
local-colon⟦TAB⟧1⟦TAB⟧63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a22
root-colon⟦TAB⟧0⟦TAB⟧
local-plain⟦TAB⟧0⟦TAB⟧
```

- Standard error: Not recorded.

The following entries decode the result character values printed in hexadecimal.

- local-colon, Tcl completion 1:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"⟦EOF⟧
```

- root-colon, Tcl completion 0:

```text-visible
⟦EOF⟧
```

- local-plain, Tcl completion 0:

```text-visible
⟦EOF⟧
```

### 8.5

- Recorded version: 8.5; full patchlevel Not recorded.
- Recorded input channel: Not recorded.
- Exit status: 0
- Standard output:

```text-visible
local-colon⟦TAB⟧1⟦TAB⟧63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a22
root-colon⟦TAB⟧0⟦TAB⟧
local-plain⟦TAB⟧0⟦TAB⟧
```

- Standard error: Not recorded.

The following entries decode the result character values printed in hexadecimal.

- local-colon, Tcl completion 1:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"⟦EOF⟧
```

- root-colon, Tcl completion 0:

```text-visible
⟦EOF⟧
```

- local-plain, Tcl completion 0:

```text-visible
⟦EOF⟧
```

### 8.6

- Recorded version: 8.6; full patchlevel Not recorded.
- Recorded input channel: Not recorded.
- Exit status: 0
- Standard output:

```text-visible
local-colon⟦TAB⟧0⟦TAB⟧
root-colon⟦TAB⟧0⟦TAB⟧
local-plain⟦TAB⟧0⟦TAB⟧
```

- Standard error: Not recorded.

The following entries decode the result character values printed in hexadecimal.

- local-colon, Tcl completion 0:

```text-visible
⟦EOF⟧
```

- root-colon, Tcl completion 0:

```text-visible
⟦EOF⟧
```

- local-plain, Tcl completion 0:

```text-visible
⟦EOF⟧
```

### 9.0

- Recorded version: 9.0; full patchlevel Not recorded.
- Recorded input channel: Not recorded.
- Exit status: 0
- Standard output:

```text-visible
local-colon⟦TAB⟧0⟦TAB⟧
root-colon⟦TAB⟧0⟦TAB⟧
local-plain⟦TAB⟧0⟦TAB⟧
```

- Standard error: Not recorded.

The following entries decode the result character values printed in hexadecimal.

- local-colon, Tcl completion 0:

```text-visible
⟦EOF⟧
```

- root-colon, Tcl completion 0:

```text-visible
⟦EOF⟧
```

- local-plain, Tcl completion 0:

```text-visible
⟦EOF⟧
```

### 9.1

- Recorded version: 9.1; full patchlevel Not recorded.
- Recorded input channel: Not recorded.
- Exit status: 0
- Standard output:

```text-visible
local-colon⟦TAB⟧0⟦TAB⟧
root-colon⟦TAB⟧0⟦TAB⟧
local-plain⟦TAB⟧0⟦TAB⟧
```

- Standard error: Not recorded.

The following entries decode the result character values printed in hexadecimal.

- local-colon, Tcl completion 0:

```text-visible
⟦EOF⟧
```

- root-colon, Tcl completion 0:

```text-visible
⟦EOF⟧
```

- local-plain, Tcl completion 0:

```text-visible
⟦EOF⟧
```

### jim

- Recorded version: jim; full patchlevel Not recorded.
- Recorded input channel: Not recorded.
- Exit status: 0
- Standard output:

```text-visible
local-colon⟦TAB⟧0⟦TAB⟧3a70
root-colon⟦TAB⟧0⟦TAB⟧3a70
local-plain⟦TAB⟧0⟦TAB⟧70
```

- Standard error: Not recorded.

The following entries decode the result character values printed in hexadecimal.

- local-colon, Tcl completion 0:

```text-visible
:p⟦EOF⟧
```

- root-colon, Tcl completion 0:

```text-visible
:p⟦EOF⟧
```

- local-plain, Tcl completion 0:

```text-visible
p⟦EOF⟧
```
