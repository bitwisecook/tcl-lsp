# Tcl naming pattern: alias-prefix-lifetime

## Question

After an interpreter alias with a fixed multiword argument is renamed, what does querying the old alias name return, and how does the renamed command apply its retained prefix to a new argument?

This distinction informs command and variable resolution in the LSP, compiler and VM. The observations cover only this case and the stated environments.

## Assumptions

The executed wrapper separates setup from the final operation. A setup failure leaves that operation unreached. Each capture uses a fresh process with the recorded interpreter; LC_ALL was C. Other assumptions were not separately recorded.

## Script

```tcl-visible

set setup [binary format H* 6e616d657370616365206576616c203a3a6e68207b70726f6320656d6974207b66697865642076616c75657d207b6c69737420246669786564202476616c75657d7d0a696e7465727020616c696173207b7d203a3a6e683a3a6c6f63616c207b7d203a3a6e683a3a656d6974207b6120627d0a72656e616d65203a3a6e683a3a6c6f63616c203a3a6e683a3a72656e616d65640a736574206465736372697074696f6e205b696e7465727020616c696173207b7d203a3a6e683a3a6c6f63616c5d]
set operation [binary format H* 6c69737420246465736372697074696f6e205b3a3a6e683a3a72656e616d6564207b6320647d5d]
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
FIELD setup_result 3a3a6e683a3a656d6974207b6120627d
FIELD setup_options {}
FIELD setup_error_code {}
FIELD setup_error_info {}
FIELD lookup_result 7b3a3a6e683a3a656d6974207b6120627d7d207b7b6120627d207b6320647d7d
FIELD lookup_options {}
FIELD lookup_error_code {}
FIELD lookup_error_info {}
TCLTEST {}
```

- Standard error: empty.

The following entries decode the FIELD payloads in standard output.

- setup_result:

```text-visible
::nh::emit {a b}⟦EOF⟧
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
{::nh::emit {a b}} {{a b} {c d}}⟦EOF⟧
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
FIELD setup_result 3a3a6e683a3a656d6974207b6120627d
FIELD setup_options 2d636f64652030202d6c6576656c2030
FIELD setup_error_code {}
FIELD setup_error_info {}
FIELD lookup_result 7b3a3a6e683a3a656d6974207b6120627d7d207b7b6120627d207b6320647d7d
FIELD lookup_options 2d636f64652030202d6c6576656c2030
FIELD lookup_error_code {}
FIELD lookup_error_info {}
TCLTEST {}
```

- Standard error: empty.

The following entries decode the FIELD payloads in standard output.

- setup_result:

```text-visible
::nh::emit {a b}⟦EOF⟧
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
{::nh::emit {a b}} {{a b} {c d}}⟦EOF⟧
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
FIELD setup_result 3a3a6e683a3a656d6974207b6120627d
FIELD setup_options 2d636f64652030202d6c6576656c2030
FIELD setup_error_code {}
FIELD setup_error_info {}
FIELD lookup_result 7b3a3a6e683a3a656d6974207b6120627d7d207b7b6120627d207b6320647d7d
FIELD lookup_options 2d636f64652030202d6c6576656c2030
FIELD lookup_error_code {}
FIELD lookup_error_info {}
TCLTEST {}
```

- Standard error: empty.

The following entries decode the FIELD payloads in standard output.

- setup_result:

```text-visible
::nh::emit {a b}⟦EOF⟧
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
{::nh::emit {a b}} {{a b} {c d}}⟦EOF⟧
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
FIELD setup_result 3a3a6e683a3a656d6974207b6120627d
FIELD setup_options 2d636f64652030202d6c6576656c2030
FIELD setup_error_code {}
FIELD setup_error_info {}
FIELD lookup_result 7b3a3a6e683a3a656d6974207b6120627d7d207b7b6120627d207b6320647d7d
FIELD lookup_options 2d636f64652030202d6c6576656c2030
FIELD lookup_error_code {}
FIELD lookup_error_info {}
TCLTEST {}
```

- Standard error: empty.

The following entries decode the FIELD payloads in standard output.

- setup_result:

```text-visible
::nh::emit {a b}⟦EOF⟧
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
{::nh::emit {a b}} {{a b} {c d}}⟦EOF⟧
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
FIELD setup_result 3a3a6e683a3a656d6974207b6120627d
FIELD setup_options 2d636f64652030202d6c6576656c2030
FIELD setup_error_code {}
FIELD setup_error_info {}
FIELD lookup_result 7b3a3a6e683a3a656d6974207b6120627d7d207b7b6120627d207b6320647d7d
FIELD lookup_options 2d636f64652030202d6c6576656c2030
FIELD lookup_error_code {}
FIELD lookup_error_info {}
TCLTEST {}
```

- Standard error: empty.

The following entries decode the FIELD payloads in standard output.

- setup_result:

```text-visible
::nh::emit {a b}⟦EOF⟧
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
{::nh::emit {a b}} {{a b} {c d}}⟦EOF⟧
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
