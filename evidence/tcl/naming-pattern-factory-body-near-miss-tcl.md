# Tcl naming pattern: factory-body-near-miss

## Question

When a loop defines procedures using a braced body containing the loop variable, is that variable substituted during definition or looked up when the generated procedure executes?

This distinction informs command and variable resolution in the LSP, compiler and VM. The observations cover only this case and the stated environments.

## Assumptions

The executed wrapper separates setup from the final operation. A setup failure leaves that operation unreached. Each capture uses a fresh process with the recorded interpreter; LC_ALL was C. Other assumptions were not separately recorded.

## Script

```tcl-visible

set setup [binary format H* 6e616d657370616365206576616c203a3a6e68207b0a20202020666f7265616368206e616d65207b72656420626c75657d207b70726f6320246e616d65207b7d207b72657475726e20246e616d657d7d0a7d]
set operation [binary format H* 3a3a6e683a3a726564]
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
FIELD setup_result {}
FIELD setup_options {}
FIELD setup_error_code {}
FIELD setup_error_info {}
FIELD lookup_result 63616e2774207265616420226e616d65223a206e6f2073756368207661726961626c65
FIELD lookup_options {}
FIELD lookup_error_code 4e4f4e45
FIELD lookup_error_info 63616e2774207265616420226e616d65223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a2272657475726e20246e616d65220a202020202870726f63656475726520223a3a6e683a3a72656422206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a223a3a6e683a3a72656422
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
can't read "name": no such variable⟦EOF⟧
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
can't read "name": no such variable
    while executing
"return $name"
    (procedure "::nh::red" line 1)
    invoked from within
"::nh::red"⟦EOF⟧
```

### Tcl 8.5.19

- Recorded version: Tcl 8.5.19
- Recorded input channel: script supplied to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
OBSERVATION setup_code 0 lookup_code 1
FIELD setup_result {}
FIELD setup_options 2d636f64652030202d6c6576656c2030
FIELD setup_error_code {}
FIELD setup_error_info {}
FIELD lookup_result 63616e2774207265616420226e616d65223a206e6f2073756368207661726961626c65
FIELD lookup_options 2d636f64652031202d6c6576656c2030202d6572726f72636f6465204e4f4e45202d6572726f72696e666f207b63616e2774207265616420226e616d65223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a2272657475726e20246e616d65220a202020202870726f63656475726520223a3a6e683a3a72656422206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a223a3a6e683a3a726564227d202d6572726f726c696e652031
FIELD lookup_error_code 4e4f4e45
FIELD lookup_error_info 63616e2774207265616420226e616d65223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a2272657475726e20246e616d65220a202020202870726f63656475726520223a3a6e683a3a72656422206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a223a3a6e683a3a72656422
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
can't read "name": no such variable⟦EOF⟧
```

- lookup_options:

```text-visible
-code 1 -level 0 -errorcode NONE -errorinfo {can't read "name": no such variable
    while executing
"return $name"
    (procedure "::nh::red" line 1)
    invoked from within
"::nh::red"} -errorline 1⟦EOF⟧
```

- lookup_error_code:

```text-visible
NONE⟦EOF⟧
```

- lookup_error_info:

```text-visible
can't read "name": no such variable
    while executing
"return $name"
    (procedure "::nh::red" line 1)
    invoked from within
"::nh::red"⟦EOF⟧
```

### Tcl 8.6.18

- Recorded version: Tcl 8.6.18
- Recorded input channel: script supplied to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
OBSERVATION setup_code 0 lookup_code 1
FIELD setup_result {}
FIELD setup_options 2d636f64652030202d6c6576656c2030
FIELD setup_error_code {}
FIELD setup_error_info {}
FIELD lookup_result 63616e2774207265616420226e616d65223a206e6f2073756368207661726961626c65
FIELD lookup_options 2d636f64652031202d6c6576656c2030202d6572726f72737461636b207b494e4e4552206c6f61645363616c6172312043414c4c203a3a6e683a3a7265647d202d6572726f72636f6465207b54434c2052454144205641524e414d457d202d6572726f72696e666f207b63616e2774207265616420226e616d65223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a2272657475726e20246e616d65220a202020202870726f63656475726520223a3a6e683a3a72656422206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a223a3a6e683a3a726564227d202d6572726f726c696e652031
FIELD lookup_error_code 54434c2052454144205641524e414d45
FIELD lookup_error_info 63616e2774207265616420226e616d65223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a2272657475726e20246e616d65220a202020202870726f63656475726520223a3a6e683a3a72656422206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a223a3a6e683a3a72656422
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
can't read "name": no such variable⟦EOF⟧
```

- lookup_options:

```text-visible
-code 1 -level 0 -errorstack {INNER loadScalar1 CALL ::nh::red} -errorcode {TCL READ VARNAME} -errorinfo {can't read "name": no such variable
    while executing
"return $name"
    (procedure "::nh::red" line 1)
    invoked from within
"::nh::red"} -errorline 1⟦EOF⟧
```

- lookup_error_code:

```text-visible
TCL READ VARNAME⟦EOF⟧
```

- lookup_error_info:

```text-visible
can't read "name": no such variable
    while executing
"return $name"
    (procedure "::nh::red" line 1)
    invoked from within
"::nh::red"⟦EOF⟧
```

### Tcl 9.0.4

- Recorded version: Tcl 9.0.4
- Recorded input channel: script supplied to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
OBSERVATION setup_code 0 lookup_code 1
FIELD setup_result {}
FIELD setup_options 2d636f64652030202d6c6576656c2030
FIELD setup_error_code {}
FIELD setup_error_info {}
FIELD lookup_result 63616e2774207265616420226e616d65223a206e6f2073756368207661726961626c65
FIELD lookup_options 2d636f64652031202d6c6576656c2030202d6572726f72737461636b207b494e4e4552206c6f61645363616c6172312043414c4c203a3a6e683a3a7265647d202d6572726f72636f6465207b54434c2052454144205641524e414d457d202d6572726f72696e666f207b63616e2774207265616420226e616d65223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a2272657475726e20246e616d65220a202020202870726f63656475726520223a3a6e683a3a72656422206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a223a3a6e683a3a726564227d202d6572726f726c696e652031
FIELD lookup_error_code 54434c2052454144205641524e414d45
FIELD lookup_error_info 63616e2774207265616420226e616d65223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a2272657475726e20246e616d65220a202020202870726f63656475726520223a3a6e683a3a72656422206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a223a3a6e683a3a72656422
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
can't read "name": no such variable⟦EOF⟧
```

- lookup_options:

```text-visible
-code 1 -level 0 -errorstack {INNER loadScalar1 CALL ::nh::red} -errorcode {TCL READ VARNAME} -errorinfo {can't read "name": no such variable
    while executing
"return $name"
    (procedure "::nh::red" line 1)
    invoked from within
"::nh::red"} -errorline 1⟦EOF⟧
```

- lookup_error_code:

```text-visible
TCL READ VARNAME⟦EOF⟧
```

- lookup_error_info:

```text-visible
can't read "name": no such variable
    while executing
"return $name"
    (procedure "::nh::red" line 1)
    invoked from within
"::nh::red"⟦EOF⟧
```

### Tcl 9.1.0

- Recorded version: Tcl 9.1.0
- Recorded input channel: script supplied to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
OBSERVATION setup_code 0 lookup_code 1
FIELD setup_result {}
FIELD setup_options 2d636f64652030202d6c6576656c2030
FIELD setup_error_code {}
FIELD setup_error_info {}
FIELD lookup_result 63616e2774207265616420226e616d65223a206e6f2073756368207661726961626c65
FIELD lookup_options 2d636f64652031202d6c6576656c2030202d6572726f72737461636b207b494e4e4552206c6f61645363616c61722043414c4c203a3a6e683a3a7265647d202d6572726f72636f6465207b54434c2052454144205641524e414d457d202d6572726f72696e666f207b63616e2774207265616420226e616d65223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a2272657475726e20246e616d65220a202020202870726f63656475726520223a3a6e683a3a72656422206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a223a3a6e683a3a726564227d202d6572726f726c696e652031
FIELD lookup_error_code 54434c2052454144205641524e414d45
FIELD lookup_error_info 63616e2774207265616420226e616d65223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a2272657475726e20246e616d65220a202020202870726f63656475726520223a3a6e683a3a72656422206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a223a3a6e683a3a72656422
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
can't read "name": no such variable⟦EOF⟧
```

- lookup_options:

```text-visible
-code 1 -level 0 -errorstack {INNER loadScalar CALL ::nh::red} -errorcode {TCL READ VARNAME} -errorinfo {can't read "name": no such variable
    while executing
"return $name"
    (procedure "::nh::red" line 1)
    invoked from within
"::nh::red"} -errorline 1⟦EOF⟧
```

- lookup_error_code:

```text-visible
TCL READ VARNAME⟦EOF⟧
```

- lookup_error_info:

```text-visible
can't read "name": no such variable
    while executing
"return $name"
    (procedure "::nh::red" line 1)
    invoked from within
"::nh::red"⟦EOF⟧
```
