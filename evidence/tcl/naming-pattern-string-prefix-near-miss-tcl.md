# Tcl naming pattern: string-prefix-near-miss

## Question

What happens when a command name containing spaces is interpolated into an eval script string rather than preserved as a list command-prefix element?

This distinction informs command and variable resolution in the LSP, compiler and VM. The observations cover only this case and the stated environments.

## Assumptions

The executed wrapper separates setup from the final operation. A setup failure leaves that operation unreached. Each capture uses a fresh process with the recorded interpreter; LC_ALL was C. Other assumptions were not separately recorded.

## Script

```tcl-visible

set setup [binary format H* 6e616d657370616365206576616c203a3a6e68207b7d0a70726f63207b3a3a6e683a3a74776f20776f7264737d207b66697865642076616c75657d207b6c69737420246669786564202476616c75657d0a73657420746172676574207b3a3a6e683a3a74776f20776f7264737d]
set operation [binary format H* 6576616c202224746172676574207b6120627d207b6320647d22]
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
OBSERVATION setup_code 0 lookup_code 1
FIELD setup_result 3a3a6e683a3a74776f20776f726473
FIELD setup_options {}
FIELD setup_error_code {}
FIELD setup_error_info {}
FIELD lookup_result 696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a74776f22
FIELD lookup_options {}
FIELD lookup_error_code 4e4f4e45
FIELD lookup_error_info 696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a74776f220a202020207768696c6520657865637574696e670a223a3a6e683a3a74776f20776f726473207b6120627d207b6320647d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c202224746172676574207b6120627d207b6320647d2222
TCLTEST {}
```

- Standard error: empty.

The following entries decode the FIELD payloads in standard output.

- setup_result:

```text-visible
::nh::two words⟦EOF⟧
```

- setup_options:

```text-visible
⟦EOF⟧
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
invalid command name "::nh::two"⟦EOF⟧
```

- lookup_options:

```text-visible
⟦EOF⟧
```

- lookup_error_code:

```text-visible
NONE⟦EOF⟧
```

- lookup_error_info:

```text-visible
invalid command name "::nh::two"
    while executing
"::nh::two words {a b} {c d}"
    ("eval" body line 1)
    invoked from within
"eval "$target {a b} {c d}""⟦EOF⟧
```

### Tcl 8.5.19

- Recorded version: Tcl 8.5.19
- Recorded input channel: script supplied to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
OBSERVATION setup_code 0 lookup_code 1
FIELD setup_result 3a3a6e683a3a74776f20776f726473
FIELD setup_options 2d636f64652030202d6c6576656c2030
FIELD setup_error_code {}
FIELD setup_error_info {}
FIELD lookup_result 696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a74776f22
FIELD lookup_options 2d636f64652031202d6c6576656c2030202d6572726f72636f6465204e4f4e45202d6572726f72696e666f207b696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a74776f220a202020207768696c6520657865637574696e670a223a3a6e683a3a74776f20776f726473207b6120627d207b6320647d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c202224746172676574207b6120627d207b6320647d22227d202d6572726f726c696e652031
FIELD lookup_error_code 4e4f4e45
FIELD lookup_error_info 696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a74776f220a202020207768696c6520657865637574696e670a223a3a6e683a3a74776f20776f726473207b6120627d207b6320647d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c202224746172676574207b6120627d207b6320647d2222
TCLTEST {}
```

- Standard error: empty.

The following entries decode the FIELD payloads in standard output.

- setup_result:

```text-visible
::nh::two words⟦EOF⟧
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
invalid command name "::nh::two"⟦EOF⟧
```

- lookup_options:

```text-visible
-code 1 -level 0 -errorcode NONE -errorinfo {invalid command name "::nh::two"
    while executing
"::nh::two words {a b} {c d}"
    ("eval" body line 1)
    invoked from within
"eval "$target {a b} {c d}""} -errorline 1⟦EOF⟧
```

- lookup_error_code:

```text-visible
NONE⟦EOF⟧
```

- lookup_error_info:

```text-visible
invalid command name "::nh::two"
    while executing
"::nh::two words {a b} {c d}"
    ("eval" body line 1)
    invoked from within
"eval "$target {a b} {c d}""⟦EOF⟧
```

### Tcl 8.6.18

- Recorded version: Tcl 8.6.18
- Recorded input channel: script supplied to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
OBSERVATION setup_code 0 lookup_code 1
FIELD setup_result 3a3a6e683a3a74776f20776f726473
FIELD setup_options 2d636f64652030202d6c6576656c2030
FIELD setup_error_code {}
FIELD setup_error_info {}
FIELD lookup_result 696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a74776f22
FIELD lookup_options 2d6572726f72636f6465207b54434c204c4f4f4b555020434f4d4d414e44203a3a6e683a3a74776f7d202d636f64652031202d6c6576656c2030202d6572726f72737461636b207b494e4e4552207b696e766f6b6553746b31203a3a6e683a3a74776f20776f726473207b6120627d207b6320647d7d7d202d6572726f72696e666f207b696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a74776f220a202020207768696c6520657865637574696e670a223a3a6e683a3a74776f20776f726473207b6120627d207b6320647d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c202224746172676574207b6120627d207b6320647d22227d202d6572726f726c696e652031
FIELD lookup_error_code 54434c204c4f4f4b555020434f4d4d414e44203a3a6e683a3a74776f
FIELD lookup_error_info 696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a74776f220a202020207768696c6520657865637574696e670a223a3a6e683a3a74776f20776f726473207b6120627d207b6320647d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c202224746172676574207b6120627d207b6320647d2222
TCLTEST {}
```

- Standard error: empty.

The following entries decode the FIELD payloads in standard output.

- setup_result:

```text-visible
::nh::two words⟦EOF⟧
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
invalid command name "::nh::two"⟦EOF⟧
```

- lookup_options:

```text-visible
-errorcode {TCL LOOKUP COMMAND ::nh::two} -code 1 -level 0 -errorstack {INNER {invokeStk1 ::nh::two words {a b} {c d}}} -errorinfo {invalid command name "::nh::two"
    while executing
"::nh::two words {a b} {c d}"
    ("eval" body line 1)
    invoked from within
"eval "$target {a b} {c d}""} -errorline 1⟦EOF⟧
```

- lookup_error_code:

```text-visible
TCL LOOKUP COMMAND ::nh::two⟦EOF⟧
```

- lookup_error_info:

```text-visible
invalid command name "::nh::two"
    while executing
"::nh::two words {a b} {c d}"
    ("eval" body line 1)
    invoked from within
"eval "$target {a b} {c d}""⟦EOF⟧
```

### Tcl 9.0.4

- Recorded version: Tcl 9.0.4
- Recorded input channel: script supplied to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
OBSERVATION setup_code 0 lookup_code 1
FIELD setup_result 3a3a6e683a3a74776f20776f726473
FIELD setup_options 2d636f64652030202d6c6576656c2030
FIELD setup_error_code {}
FIELD setup_error_info {}
FIELD lookup_result 696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a74776f22
FIELD lookup_options 2d6572726f72636f6465207b54434c204c4f4f4b555020434f4d4d414e44203a3a6e683a3a74776f7d202d636f64652031202d6c6576656c2030202d6572726f72737461636b207b494e4e4552207b696e766f6b6553746b31203a3a6e683a3a74776f20776f726473207b6120627d207b6320647d7d7d202d6572726f72696e666f207b696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a74776f220a202020207768696c6520657865637574696e670a223a3a6e683a3a74776f20776f726473207b6120627d207b6320647d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c202224746172676574207b6120627d207b6320647d22227d202d6572726f726c696e652031
FIELD lookup_error_code 54434c204c4f4f4b555020434f4d4d414e44203a3a6e683a3a74776f
FIELD lookup_error_info 696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a74776f220a202020207768696c6520657865637574696e670a223a3a6e683a3a74776f20776f726473207b6120627d207b6320647d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c202224746172676574207b6120627d207b6320647d2222
TCLTEST {}
```

- Standard error: empty.

The following entries decode the FIELD payloads in standard output.

- setup_result:

```text-visible
::nh::two words⟦EOF⟧
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
invalid command name "::nh::two"⟦EOF⟧
```

- lookup_options:

```text-visible
-errorcode {TCL LOOKUP COMMAND ::nh::two} -code 1 -level 0 -errorstack {INNER {invokeStk1 ::nh::two words {a b} {c d}}} -errorinfo {invalid command name "::nh::two"
    while executing
"::nh::two words {a b} {c d}"
    ("eval" body line 1)
    invoked from within
"eval "$target {a b} {c d}""} -errorline 1⟦EOF⟧
```

- lookup_error_code:

```text-visible
TCL LOOKUP COMMAND ::nh::two⟦EOF⟧
```

- lookup_error_info:

```text-visible
invalid command name "::nh::two"
    while executing
"::nh::two words {a b} {c d}"
    ("eval" body line 1)
    invoked from within
"eval "$target {a b} {c d}""⟦EOF⟧
```

### Tcl 9.1.0

- Recorded version: Tcl 9.1.0
- Recorded input channel: script supplied to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
OBSERVATION setup_code 0 lookup_code 1
FIELD setup_result 3a3a6e683a3a74776f20776f726473
FIELD setup_options 2d636f64652030202d6c6576656c2030
FIELD setup_error_code {}
FIELD setup_error_info {}
FIELD lookup_result 696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a74776f22
FIELD lookup_options 2d6572726f72636f6465207b54434c204c4f4f4b555020434f4d4d414e44203a3a6e683a3a74776f7d202d636f64652031202d6c6576656c2030202d6572726f72737461636b207b494e4e4552207b696e766f6b6553746b203a3a6e683a3a74776f20776f726473207b6120627d207b6320647d7d7d202d6572726f72696e666f207b696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a74776f220a202020207768696c6520657865637574696e670a223a3a6e683a3a74776f20776f726473207b6120627d207b6320647d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c202224746172676574207b6120627d207b6320647d22227d202d6572726f726c696e652031
FIELD lookup_error_code 54434c204c4f4f4b555020434f4d4d414e44203a3a6e683a3a74776f
FIELD lookup_error_info 696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a74776f220a202020207768696c6520657865637574696e670a223a3a6e683a3a74776f20776f726473207b6120627d207b6320647d220a2020202028226576616c2220626f6479206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a226576616c202224746172676574207b6120627d207b6320647d2222
TCLTEST {}
```

- Standard error: empty.

The following entries decode the FIELD payloads in standard output.

- setup_result:

```text-visible
::nh::two words⟦EOF⟧
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
invalid command name "::nh::two"⟦EOF⟧
```

- lookup_options:

```text-visible
-errorcode {TCL LOOKUP COMMAND ::nh::two} -code 1 -level 0 -errorstack {INNER {invokeStk ::nh::two words {a b} {c d}}} -errorinfo {invalid command name "::nh::two"
    while executing
"::nh::two words {a b} {c d}"
    ("eval" body line 1)
    invoked from within
"eval "$target {a b} {c d}""} -errorline 1⟦EOF⟧
```

- lookup_error_code:

```text-visible
TCL LOOKUP COMMAND ::nh::two⟦EOF⟧
```

- lookup_error_info:

```text-visible
invalid command name "::nh::two"
    while executing
"::nh::two words {a b} {c d}"
    ("eval" body line 1)
    invoked from within
"eval "$target {a b} {c d}""⟦EOF⟧
```
