# Tcl naming pattern: oo-self-prefix

## Question

Can a TclOO method return a list prefix containing its own object command and a multiword argument for later method invocation?

This distinction informs command and variable resolution in the LSP, compiler and VM. The observations cover only this case and the stated environments.

## Assumptions

The executed wrapper separates setup from the final operation. A setup failure leaves that operation unreached. Each capture uses a fresh process with the recorded interpreter; LC_ALL was C. Other assumptions were not separately recorded.

## Script

```tcl-visible

set setup [binary format H* 6f6f3a3a636c61737320637265617465203a3a6e68636c617373207b0a202020206d6574686f6420656d6974207b76616c75657d207b6c697374206f6b202476616c75657d0a202020206d6574686f6420707265666978207b7d207b6c697374205b73656c665d20656d6974207b6120627d7d0a7d0a736574206f626a656374205b3a3a6e68636c617373206e65775d]
set operation [binary format H* 6576616c205b246f626a656374207072656669785d]
set catch_command catch
set modern [expr {[package vcompare [info patchlevel] 8.5] >= 0}]
set setup_options {}
if {$modern} {
    set setup_code [$catch_command $setup setup_result setup_options]
} else {
    set setup_code [$catch_command $setup setup_result]
}
set setup_error_code {}
set setup_error_info {}
if {$setup_code && [info exists ::errorCode]} {set setup_error_code $::errorCode; set setup_error_info $::errorInfo}
if {$setup_code} {
    set lookup_code not-run
    set lookup_result {}
    set lookup_options {}
} elseif {$modern} {
    set lookup_code [$catch_command $operation lookup_result lookup_options]
} else {
    set lookup_code [$catch_command $operation lookup_result]
    set lookup_options {}
}
set lookup_error_code {}
set lookup_error_info {}
if {$lookup_code eq "1" && [info exists ::errorCode]} {set lookup_error_code $::errorCode; set lookup_error_info $::errorInfo}
puts [list OBSERVATION setup_code $setup_code lookup_code $lookup_code]
foreach field {setup_result setup_options setup_error_code setup_error_info lookup_result lookup_options lookup_error_code lookup_error_info} {
    binary scan [set $field] H* bytes
    puts [list FIELD $field $bytes]
}
puts [list TCLTEST [expr {[info exists ::tcltest::numTests] ? [array get ::tcltest::numTests] : {}}]]
```

## Results

### Tcl 8.4.20

- Recorded version: Tcl 8.4.20
- Recorded input channel: script supplied to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
OBSERVATION setup_code 1 lookup_code not-run
FIELD setup_result 696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
FIELD setup_options {}
FIELD setup_error_code 4e4f4e45
FIELD setup_error_info 696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c617373220a202020207768696c6520657865637574696e670a226f6f3a3a636c61737320637265617465203a3a6e68636c617373207b0a202020206d6574686f6420656d6974207b76616c75657d207b6c697374206f6b202476616c75657d0a202020206d6574686f6420707265666978207b7d207b6c697374205b73656c665d20656d6974207b6120627d7d0a7d22
FIELD lookup_result {}
FIELD lookup_options {}
FIELD lookup_error_code {}
FIELD lookup_error_info {}
TCLTEST {}
```

- Standard error: empty.

The intended final operation was not reached.

The following entries decode the FIELD payloads in standard output.

- setup_result:

```text-visible
invalid command name "oo::class"⟦EOF⟧
```

- setup_options:

```text-visible
⟦EOF⟧
```

- setup_error_code:

```text-visible
NONE⟦EOF⟧
```

- setup_error_info:

```text-visible
invalid command name "oo::class"
    while executing
"oo::class create ::nhclass {
    method emit {value} {list ok $value}
    method prefix {} {list [self] emit {a b}}
}"⟦EOF⟧
```

- lookup_result:

```text-visible
⟦EOF⟧
```

- lookup_options:

```text-visible
⟦EOF⟧
```

- lookup_error_code:

```text-visible
⟦EOF⟧
```

- lookup_error_info:

```text-visible
⟦EOF⟧
```

### Tcl 8.5.19

- Recorded version: Tcl 8.5.19
- Recorded input channel: script supplied to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
OBSERVATION setup_code 1 lookup_code not-run
FIELD setup_result 696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
FIELD setup_options 2d636f64652031202d6c6576656c2030202d6572726f72636f6465204e4f4e45202d6572726f72696e666f207b696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c617373220a202020207768696c6520657865637574696e670a226f6f3a3a636c61737320637265617465203a3a6e68636c617373207b0a202020206d6574686f6420656d6974207b76616c75657d207b6c697374206f6b202476616c75657d0a202020206d6574686f6420707265666978207b7d207b6c697374205b73656c665d20656d6974207b6120627d7d0a7d227d202d6572726f726c696e652031
FIELD setup_error_code 4e4f4e45
FIELD setup_error_info 696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c617373220a202020207768696c6520657865637574696e670a226f6f3a3a636c61737320637265617465203a3a6e68636c617373207b0a202020206d6574686f6420656d6974207b76616c75657d207b6c697374206f6b202476616c75657d0a202020206d6574686f6420707265666978207b7d207b6c697374205b73656c665d20656d6974207b6120627d7d0a7d22
FIELD lookup_result {}
FIELD lookup_options {}
FIELD lookup_error_code {}
FIELD lookup_error_info {}
TCLTEST {}
```

- Standard error: empty.

The intended final operation was not reached.

The following entries decode the FIELD payloads in standard output.

- setup_result:

```text-visible
invalid command name "oo::class"⟦EOF⟧
```

- setup_options:

```text-visible
-code 1 -level 0 -errorcode NONE -errorinfo {invalid command name "oo::class"
    while executing
"oo::class create ::nhclass {
    method emit {value} {list ok $value}
    method prefix {} {list [self] emit {a b}}
}"} -errorline 1⟦EOF⟧
```

- setup_error_code:

```text-visible
NONE⟦EOF⟧
```

- setup_error_info:

```text-visible
invalid command name "oo::class"
    while executing
"oo::class create ::nhclass {
    method emit {value} {list ok $value}
    method prefix {} {list [self] emit {a b}}
}"⟦EOF⟧
```

- lookup_result:

```text-visible
⟦EOF⟧
```

- lookup_options:

```text-visible
⟦EOF⟧
```

- lookup_error_code:

```text-visible
⟦EOF⟧
```

- lookup_error_info:

```text-visible
⟦EOF⟧
```

### Tcl 8.6.18

- Recorded version: Tcl 8.6.18
- Recorded input channel: script supplied to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
OBSERVATION setup_code 0 lookup_code 0
FIELD setup_result 3a3a6f6f3a3a4f626a3132
FIELD setup_options 2d636f64652030202d6c6576656c2030
FIELD setup_error_code {}
FIELD setup_error_info {}
FIELD lookup_result 6f6b207b6120627d
FIELD lookup_options 2d636f64652030202d6c6576656c2030
FIELD lookup_error_code {}
FIELD lookup_error_info {}
TCLTEST {}
```

- Standard error: empty.

The following entries decode the FIELD payloads in standard output.

- setup_result:

```text-visible
::oo::Obj12⟦EOF⟧
```

- setup_options:

```text-visible
-code 0 -level 0⟦EOF⟧
```

- setup_error_code:

```text-visible
⟦EOF⟧
```

- setup_error_info:

```text-visible
⟦EOF⟧
```

- lookup_result:

```text-visible
ok {a b}⟦EOF⟧
```

- lookup_options:

```text-visible
-code 0 -level 0⟦EOF⟧
```

- lookup_error_code:

```text-visible
⟦EOF⟧
```

- lookup_error_info:

```text-visible
⟦EOF⟧
```

### Tcl 9.0.4

- Recorded version: Tcl 9.0.4
- Recorded input channel: script supplied to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
OBSERVATION setup_code 0 lookup_code 0
FIELD setup_result 3a3a6f6f3a3a4f626a3232
FIELD setup_options 2d636f64652030202d6c6576656c2030
FIELD setup_error_code {}
FIELD setup_error_info {}
FIELD lookup_result 6f6b207b6120627d
FIELD lookup_options 2d636f64652030202d6c6576656c2030
FIELD lookup_error_code {}
FIELD lookup_error_info {}
TCLTEST {}
```

- Standard error: empty.

The following entries decode the FIELD payloads in standard output.

- setup_result:

```text-visible
::oo::Obj22⟦EOF⟧
```

- setup_options:

```text-visible
-code 0 -level 0⟦EOF⟧
```

- setup_error_code:

```text-visible
⟦EOF⟧
```

- setup_error_info:

```text-visible
⟦EOF⟧
```

- lookup_result:

```text-visible
ok {a b}⟦EOF⟧
```

- lookup_options:

```text-visible
-code 0 -level 0⟦EOF⟧
```

- lookup_error_code:

```text-visible
⟦EOF⟧
```

- lookup_error_info:

```text-visible
⟦EOF⟧
```

### Tcl 9.1.0

- Recorded version: Tcl 9.1.0
- Recorded input channel: script supplied to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
OBSERVATION setup_code 0 lookup_code 0
FIELD setup_result 3a3a6f6f3a3a4f626a3232
FIELD setup_options 2d636f64652030202d6c6576656c2030
FIELD setup_error_code {}
FIELD setup_error_info {}
FIELD lookup_result 6f6b207b6120627d
FIELD lookup_options 2d636f64652030202d6c6576656c2030
FIELD lookup_error_code {}
FIELD lookup_error_info {}
TCLTEST {}
```

- Standard error: empty.

The following entries decode the FIELD payloads in standard output.

- setup_result:

```text-visible
::oo::Obj22⟦EOF⟧
```

- setup_options:

```text-visible
-code 0 -level 0⟦EOF⟧
```

- setup_error_code:

```text-visible
⟦EOF⟧
```

- setup_error_info:

```text-visible
⟦EOF⟧
```

- lookup_result:

```text-visible
ok {a b}⟦EOF⟧
```

- lookup_options:

```text-visible
-code 0 -level 0⟦EOF⟧
```

- lookup_error_code:

```text-visible
⟦EOF⟧
```

- lookup_error_info:

```text-visible
⟦EOF⟧
```
