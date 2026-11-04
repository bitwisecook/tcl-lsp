# Tcl naming pattern: finite-factory

## Question

When a loop constructs procedure bodies as lists containing the current loop value, do the resulting commands retain their respective values?

This distinction informs command and variable resolution in the LSP, compiler and VM. The observations cover only this case and the stated environments.

## Assumptions

The executed wrapper separates setup from the final operation. A setup failure leaves that operation unreached. Each capture uses a fresh process with the recorded interpreter; LC_ALL was C. The Jim wrapper also sources the test helper named in the script; the result depends on that helper. Other assumptions were not separately recorded.

## Script

```tcl-visible
source [binary format H* 2f776f726b73706163652f74636c2d6c73702f746d702f6a696d74636c2d6f7261636c652f74657374732f74657374696e672e74636c]
set setup [binary format H* 6e616d657370616365206576616c203a3a6e68207b0a20202020666f7265616368206e616d65207b72656420626c75657d207b70726f6320246e616d65207b7d205b6c6973742072657475726e20246e616d655d7d0a7d]
set question [binary format H* 6c697374205b3a3a6e683a3a7265645d205b3a3a6e683a3a626c75655d]
set setup_code [catch $setup setup_result setup_options]
set question_code not-run
set question_result {}
set question_options {}
if {!$setup_code} {
    set question_code [catch $question question_result question_options]
}
foreach field {setup_code setup_result setup_options question_code question_result question_options} {
    binary scan [set $field] H* bytes
    puts [list FIELD $field $bytes]
}
puts [list JIMTEST [array get ::testinfo]]
```

## Results

### Jim 0.84-9-g5bac7c9

- Recorded version: Jim 0.84-9-g5bac7c9
- Recorded input channel: script supplied to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
FIELD setup_code 30
FIELD setup_result {}
FIELD setup_options 2d636f64652030202d6c6576656c2030
FIELD question_code 30
FIELD question_result 72656420626c7565
FIELD question_options 2d636f64652030202d6c6576656c2030
JIMTEST {verbose 0 numpass 0 stoponerror 0 template 0 numfail 0 numskip 0 numtests 0 reported 0 failed {} source input.tcl}
```

- Standard error: empty.

The following entries decode the FIELD payloads in standard output.

- setup_code:

```text-visible
0⟦EOF⟧
```

- setup_result:

```text-visible
⟦EOF⟧
```

- setup_options:

```text-visible
-code 0 -level 0⟦EOF⟧
```

- question_code:

```text-visible
0⟦EOF⟧
```

- question_result:

```text-visible
red blue⟦EOF⟧
```

- question_options:

```text-visible
-code 0 -level 0⟦EOF⟧
```
