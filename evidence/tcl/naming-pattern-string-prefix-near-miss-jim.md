# Tcl naming pattern: string-prefix-near-miss

## Question

What happens when a command name containing spaces is interpolated into an eval script string rather than preserved as a list command-prefix element?

This distinction informs command and variable resolution in the LSP, compiler and VM. The observations cover only this case and the stated environments.

## Assumptions

The executed wrapper separates setup from the final operation. A setup failure leaves that operation unreached. Each capture uses a fresh process with the recorded interpreter; LC_ALL was C. The Jim wrapper also sources the test helper named in the script; the result depends on that helper. Other assumptions were not separately recorded.

## Script

```tcl-visible
source [binary format H* 2f776f726b73706163652f74636c2d6c73702f746d702f6a696d74636c2d6f7261636c652f74657374732f74657374696e672e74636c]
set setup [binary format H* 6e616d657370616365206576616c203a3a6e68207b7d0a70726f63207b3a3a6e683a3a74776f20776f7264737d207b66697865642076616c75657d207b6c69737420246669786564202476616c75657d0a73657420746172676574207b3a3a6e683a3a74776f20776f7264737d]
set question [binary format H* 6576616c202224746172676574207b6120627d207b6320647d22]
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
FIELD setup_result 3a3a6e683a3a74776f20776f726473
FIELD setup_options 2d636f64652030202d6c6576656c2030
FIELD question_code 31
FIELD question_result 696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a74776f22
FIELD question_options 2d636f64652031202d6c6576656c2030202d6572726f72696e666f207b7b7d207b7d2031207b7d7d202d6572726f72636f6465204e4f4e45
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
::nh::two words⟦EOF⟧
```

- setup_options:

```text-visible
-code 0 -level 0⟦EOF⟧
```

- question_code:

```text-visible
1⟦EOF⟧
```

- question_result:

```text-visible
invalid command name "::nh::two"⟦EOF⟧
```

- question_options:

```text-visible
-code 1 -level 0 -errorinfo {{} {} 1 {}} -errorcode NONE⟦EOF⟧
```
