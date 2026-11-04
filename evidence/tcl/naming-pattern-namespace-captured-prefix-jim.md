# Tcl naming pattern: namespace-captured-prefix

## Question

Does a namespace code prefix retain the defining namespace when evaluated elsewhere, including after a same-named global command is defined, and preserve its argument elements?

This distinction informs command and variable resolution in the LSP, compiler and VM. The observations cover only this case and the stated environments.

## Assumptions

The executed wrapper separates setup from the final operation. A setup failure leaves that operation unreached. Each capture uses a fresh process with the recorded interpreter; LC_ALL was C. The Jim wrapper also sources the test helper named in the script; the result depends on that helper. Other assumptions were not separately recorded.

## Script

```tcl-visible
source [binary format H* 2f776f726b73706163652f74636c2d6c73702f746d702f6a696d74636c2d6f7261636c652f74657374732f74657374696e672e74636c]
set setup [binary format H* 6e616d657370616365206576616c203a3a6e68207b0a2020202070726f6320656d6974207b76616c756520617267737d207b6c697374206c6f63616c202476616c75652024617267737d0a2020202073657420707265666978205b6e616d65737061636520636f6465205b6c69737420656d6974207b6120627d5d5d0a7d0a70726f6320656d6974207b617267737d207b72657475726e20676c6f62616c7d]
set question [binary format H* 6576616c205b6c696e7365727420243a3a6e683a3a70726566697820656e64207b6320647d5d]
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
FIELD setup_result 656d6974
FIELD setup_options 2d636f64652030202d6c6576656c2030
FIELD question_code 31
FIELD question_result 63616e2774207265616420223a3a6e683a3a707265666978223a206e6f2073756368207661726961626c65
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
emit⟦EOF⟧
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
can't read "::nh::prefix": no such variable⟦EOF⟧
```

- question_options:

```text-visible
-code 1 -level 0 -errorinfo {{} {} 1 {}} -errorcode NONE⟦EOF⟧
```
