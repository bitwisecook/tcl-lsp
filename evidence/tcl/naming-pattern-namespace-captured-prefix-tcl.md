# Tcl naming pattern: namespace-captured-prefix

## Question

Does a namespace code prefix retain the defining namespace when evaluated elsewhere, including after a same-named global command is defined, and preserve its argument elements?

This distinction informs command and variable resolution in the LSP, compiler and VM. The observations cover only this case and the stated environments.

## Assumptions

The executed wrapper separates setup from the final operation. A setup failure leaves that operation unreached. Each capture uses a fresh process with the recorded interpreter; LC_ALL was C. Other assumptions were not separately recorded.

## Script

```tcl-visible

set setup [binary format H* 6e616d657370616365206576616c203a3a6e68207b0a2020202070726f6320656d6974207b76616c756520617267737d207b6c697374206c6f63616c202476616c75652024617267737d0a2020202073657420707265666978205b6e616d65737061636520636f6465205b6c69737420656d6974207b6120627d5d5d0a7d0a70726f6320656d6974207b617267737d207b72657475726e20676c6f62616c7d]
set operation [binary format H* 6576616c205b6c696e7365727420243a3a6e683a3a70726566697820656e64207b6320647d5d]
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
OBSERVATION setup_code 0 lookup_code 0
FIELD setup_result {}
FIELD setup_options {}
FIELD setup_error_code {}
FIELD setup_error_info {}
FIELD lookup_result 6c6f63616c207b6120627d207b7b6320647d7d
FIELD lookup_options {}
FIELD lookup_error_code {}
FIELD lookup_error_info {}
TCLTEST {}
```

- Standard error: empty.

The following entries decode the FIELD payloads in standard output.

- setup_result:

```text-visible
⟦EOF⟧
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
local {a b} {{c d}}⟦EOF⟧
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
OBSERVATION setup_code 0 lookup_code 0
FIELD setup_result {}
FIELD setup_options 2d636f64652030202d6c6576656c2030
FIELD setup_error_code {}
FIELD setup_error_info {}
FIELD lookup_result 6c6f63616c207b6120627d207b7b6320647d7d
FIELD lookup_options 2d636f64652030202d6c6576656c2030
FIELD lookup_error_code {}
FIELD lookup_error_info {}
TCLTEST {}
```

- Standard error: empty.

The following entries decode the FIELD payloads in standard output.

- setup_result:

```text-visible
⟦EOF⟧
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
local {a b} {{c d}}⟦EOF⟧
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

### Tcl 8.6.18

- Recorded version: Tcl 8.6.18
- Recorded input channel: script supplied to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
OBSERVATION setup_code 0 lookup_code 0
FIELD setup_result {}
FIELD setup_options 2d636f64652030202d6c6576656c2030
FIELD setup_error_code {}
FIELD setup_error_info {}
FIELD lookup_result 6c6f63616c207b6120627d207b7b6320647d7d
FIELD lookup_options 2d636f64652030202d6c6576656c2030
FIELD lookup_error_code {}
FIELD lookup_error_info {}
TCLTEST {}
```

- Standard error: empty.

The following entries decode the FIELD payloads in standard output.

- setup_result:

```text-visible
⟦EOF⟧
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
local {a b} {{c d}}⟦EOF⟧
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
FIELD setup_result {}
FIELD setup_options 2d636f64652030202d6c6576656c2030
FIELD setup_error_code {}
FIELD setup_error_info {}
FIELD lookup_result 6c6f63616c207b6120627d207b7b6320647d7d
FIELD lookup_options 2d636f64652030202d6c6576656c2030
FIELD lookup_error_code {}
FIELD lookup_error_info {}
TCLTEST {}
```

- Standard error: empty.

The following entries decode the FIELD payloads in standard output.

- setup_result:

```text-visible
⟦EOF⟧
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
local {a b} {{c d}}⟦EOF⟧
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
FIELD setup_result {}
FIELD setup_options 2d636f64652030202d6c6576656c2030
FIELD setup_error_code {}
FIELD setup_error_info {}
FIELD lookup_result 6c6f63616c207b6120627d207b7b6320647d7d
FIELD lookup_options 2d636f64652030202d6c6576656c2030
FIELD lookup_error_code {}
FIELD lookup_error_info {}
TCLTEST {}
```

- Standard error: empty.

The following entries decode the FIELD payloads in standard output.

- setup_result:

```text-visible
⟦EOF⟧
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
local {a b} {{c d}}⟦EOF⟧
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
