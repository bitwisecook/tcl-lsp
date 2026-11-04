# Tcl naming pattern: ensemble-prefix-map

## Question

Does an ensemble subcommand mapping preserve the elements of a list command prefix, including a fixed argument containing a space, when passing a new argument?

This distinction informs command and variable resolution in the LSP, compiler and VM. The observations cover only this case and the stated environments.

## Assumptions

The executed wrapper separates setup from the final operation. A setup failure leaves that operation unreached. Each capture uses a fresh process with the recorded interpreter; LC_ALL was C. The Jim wrapper also sources the test helper named in the script; the result depends on that helper. Other assumptions were not separately recorded.

## Script

```tcl-visible
source [binary format H* 2f776f726b73706163652f74636c2d6c73702f746d702f6a696d74636c2d6f7261636c652f74657374732f74657374696e672e74636c]
set setup [binary format H* 6e616d657370616365206576616c203a3a6e68207b0a2020202070726f6320656d6974207b66697865642076616c75657d207b6c69737420246669786564202476616c75657d0a202020206e616d65737061636520656e73656d626c6520637265617465202d636f6d6d616e6420666163616465202d6d6170205b6c6973742073656e64205b6c697374203a3a6e683a3a656d6974207b6120627d5d5d0a7d]
set question [binary format H* 3a3a6e683a3a6661636164652073656e64207b6320647d]
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
FIELD setup_code 31
FIELD setup_result 77726f6e67202320617267733a2073686f756c642062652022656e73656d626c6520636f6d6d616e64203f2d6175746f6d6170207072656669783f22
FIELD setup_options 2d636f64652031202d6c6576656c2030202d6572726f72696e666f207b7b6e616d65737061636520656e73656d626c657d206e7368656c7065722e74636c20313538207b656e73656d626c65206e68202d6175746f6d6170206e683a3a202d636f6d6d616e6420666163616465202d6d6170207b73656e64207b3a3a6e683a3a656d6974207b6120627d7d7d7d207b7d207b7d2033207b7b6e616d65737061636520656e73656d626c657d20637265617465202d636f6d6d616e6420666163616465202d6d6170207b73656e64207b3a3a6e683a3a656d6974207b6120627d7d7d7d7d202d6572726f72636f6465204e4f4e45
FIELD question_code 6e6f742d72756e
FIELD question_result {}
FIELD question_options {}
JIMTEST {verbose 0 numpass 0 stoponerror 0 template 0 numfail 0 numskip 0 numtests 0 reported 0 failed {} source input.tcl}
```

- Standard error: empty.

The intended final operation was not reached.

The following entries decode the FIELD payloads in standard output.

- setup_code:

```text-visible
1⟦EOF⟧
```

- setup_result:

```text-visible
wrong # args: should be "ensemble command ?-automap prefix?"⟦EOF⟧
```

- setup_options:

```text-visible
-code 1 -level 0 -errorinfo {{namespace ensemble} nshelper.tcl 158 {ensemble nh -automap nh:: -command facade -map {send {::nh::emit {a b}}}} {} {} 3 {{namespace ensemble} create -command facade -map {send {::nh::emit {a b}}}}} -errorcode NONE⟦EOF⟧
```

- question_code:

```text-visible
not-run⟦EOF⟧
```

- question_result:

```text-visible
⟦EOF⟧
```

- question_options:

```text-visible
⟦EOF⟧
```
