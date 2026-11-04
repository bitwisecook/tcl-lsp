# Tcl naming pattern: keyed-variable-link

## Question

Can upvar link to a namespace array whose constructed name contains a space, and does writing an element through the link update the intended namespace variable?

This distinction informs command and variable resolution in the LSP, compiler and VM. The observations cover only this case and the stated environments.

## Assumptions

The executed wrapper separates setup from the final operation. A setup failure leaves that operation unreached. Each capture uses a fresh process with the recorded interpreter; LC_ALL was C. Other assumptions were not separately recorded.

## Script

```tcl-visible

set setup [binary format H* 6e616d657370616365206576616c203a3a6e68207b0a20202020736574207b73746174652e6120622876616c7565297d206f726967696e616c0a2020202070726f63206368616e6765207b6e616d657d207b0a20202020202020207570766172203a3a6e683a3a246e616d652073746174650a20202020202020207365742073746174652876616c756529206368616e6765640a202020207d0a7d0a3a3a6e683a3a6368616e6765207b73746174652e6120627d]
set operation [binary format H* 736574207b3a3a6e683a3a73746174652e6120622876616c7565297d]
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
FIELD setup_result 6368616e676564
FIELD setup_options {}
FIELD setup_error_code {}
FIELD setup_error_info {}
FIELD lookup_result 6368616e676564
FIELD lookup_options {}
FIELD lookup_error_code {}
FIELD lookup_error_info {}
TCLTEST {}
```

- Standard error: empty.

The following entries decode the FIELD payloads in standard output.

- setup_result:

```text-visible
changed⟦EOF⟧
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
changed⟦EOF⟧
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
FIELD setup_result 6368616e676564
FIELD setup_options 2d636f64652030202d6c6576656c2030
FIELD setup_error_code {}
FIELD setup_error_info {}
FIELD lookup_result 6368616e676564
FIELD lookup_options 2d636f64652030202d6c6576656c2030
FIELD lookup_error_code {}
FIELD lookup_error_info {}
TCLTEST {}
```

- Standard error: empty.

The following entries decode the FIELD payloads in standard output.

- setup_result:

```text-visible
changed⟦EOF⟧
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
changed⟦EOF⟧
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
FIELD setup_result 6368616e676564
FIELD setup_options 2d636f64652030202d6c6576656c2030
FIELD setup_error_code {}
FIELD setup_error_info {}
FIELD lookup_result 6368616e676564
FIELD lookup_options 2d636f64652030202d6c6576656c2030
FIELD lookup_error_code {}
FIELD lookup_error_info {}
TCLTEST {}
```

- Standard error: empty.

The following entries decode the FIELD payloads in standard output.

- setup_result:

```text-visible
changed⟦EOF⟧
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
changed⟦EOF⟧
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
FIELD setup_result 6368616e676564
FIELD setup_options 2d636f64652030202d6c6576656c2030
FIELD setup_error_code {}
FIELD setup_error_info {}
FIELD lookup_result 6368616e676564
FIELD lookup_options 2d636f64652030202d6c6576656c2030
FIELD lookup_error_code {}
FIELD lookup_error_info {}
TCLTEST {}
```

- Standard error: empty.

The following entries decode the FIELD payloads in standard output.

- setup_result:

```text-visible
changed⟦EOF⟧
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
changed⟦EOF⟧
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
FIELD setup_result 6368616e676564
FIELD setup_options 2d636f64652030202d6c6576656c2030
FIELD setup_error_code {}
FIELD setup_error_info {}
FIELD lookup_result 6368616e676564
FIELD lookup_options 2d636f64652030202d6c6576656c2030
FIELD lookup_error_code {}
FIELD lookup_error_info {}
TCLTEST {}
```

- Standard error: empty.

The following entries decode the FIELD payloads in standard output.

- setup_result:

```text-visible
changed⟦EOF⟧
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
changed⟦EOF⟧
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
