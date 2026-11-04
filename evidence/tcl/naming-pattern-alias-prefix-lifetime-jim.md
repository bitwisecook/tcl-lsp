# Tcl naming pattern: alias-prefix-lifetime

## Question

After an interpreter alias with a fixed multiword argument is renamed, what does querying the old alias name return, and how does the renamed command apply its retained prefix to a new argument?

This distinction informs command and variable resolution in the LSP, compiler and VM. The observations cover only this case and the stated environments.

## Assumptions

The executed wrapper separates setup from the final operation. A setup failure leaves that operation unreached. Each capture uses a fresh process with the recorded interpreter; LC_ALL was C. The Jim wrapper also sources the test helper named in the script; the result depends on that helper. Other assumptions were not separately recorded.

## Script

```tcl-visible
source [binary format H* 2f776f726b73706163652f74636c2d6c73702f746d702f6a696d74636c2d6f7261636c652f74657374732f74657374696e672e74636c]
set setup [binary format H* 6e616d657370616365206576616c203a3a6e68207b70726f6320656d6974207b66697865642076616c75657d207b6c69737420246669786564202476616c75657d7d0a696e7465727020616c696173207b7d203a3a6e683a3a6c6f63616c207b7d203a3a6e683a3a656d6974207b6120627d0a72656e616d65203a3a6e683a3a6c6f63616c203a3a6e683a3a72656e616d65640a736574206465736372697074696f6e205b696e7465727020616c696173207b7d203a3a6e683a3a6c6f63616c5d]
set question [binary format H* 6c69737420246465736372697074696f6e205b3a3a6e683a3a72656e616d6564207b6320647d5d]
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
FIELD setup_result 77726f6e67202320617267733a2073686f756c642062652022696e7465727022
FIELD setup_options 2d636f64652031202d6c6576656c2030202d6572726f72696e666f207b7b7d207b7d2032207b696e7465727020616c696173207b7d203a3a6e683a3a6c6f63616c207b7d203a3a6e683a3a656d6974207b6120627d7d7d202d6572726f72636f6465204e4f4e45
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
wrong # args: should be "interp"⟦EOF⟧
```

- setup_options:

```text-visible
-code 1 -level 0 -errorinfo {{} {} 2 {interp alias {} ::nh::local {} ::nh::emit {a b}}} -errorcode NONE⟦EOF⟧
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
