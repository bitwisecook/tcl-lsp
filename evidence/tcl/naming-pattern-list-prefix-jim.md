# Tcl naming pattern: list-prefix

## Question

Does a list command prefix preserve a command name containing spaces and its fixed multiword argument when eval invokes it with an additional argument?

This distinction informs command and variable resolution in the LSP, compiler and VM. The observations cover only this case and the stated environments.

## Assumptions

The executed wrapper separates setup from the final operation. A setup failure leaves that operation unreached. Each capture uses a fresh process with the recorded interpreter; LC_ALL was C. The Jim wrapper also sources the test helper named in the script; the result depends on that helper. Other assumptions were not separately recorded.

## Script

```tcl-visible
source [binary format H* 2f776f726b73706163652f74636c2d6c73702f746d702f6a696d74636c2d6f7261636c652f74657374732f74657374696e672e74636c]
set setup [binary format H* 6e616d657370616365206576616c203a3a6e68207b7d0a70726f63207b3a3a6e683a3a74776f20776f7264737d207b66697865642076616c75657d207b6c69737420246669786564202476616c75657d0a73657420746172676574207b3a3a6e683a3a74776f20776f7264737d0a73657420707265666978205b6c6973742024746172676574207b6120627d5d]
set question [binary format H* 6576616c205b6c696e73657274202470726566697820656e64207b6320647d5d]
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
FIELD setup_result 7b3a3a6e683a3a74776f20776f7264737d207b6120627d
FIELD setup_options 2d636f64652030202d6c6576656c2030
FIELD question_code 30
FIELD question_result 7b6120627d207b6320647d
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
{::nh::two words} {a b}⟦EOF⟧
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
{a b} {c d}⟦EOF⟧
```

- question_options:

```text-visible
-code 0 -level 0⟦EOF⟧
```
